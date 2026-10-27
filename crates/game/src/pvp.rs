//! Players against players, the scoreboard and teams (Lote 7 of plan/o-que-falta.md).
//!
//! PvP is a world rule (`/regra pvp sim`), off by default so friends building together don't
//! hurt each other by accident. The host decides every hit: a guest's swing at another player
//! goes to the host, which checks the rule, the distance and the teams, then hurts the target.
//! Player hits are not scaled by difficulty (they still go through shields and armor).
//!
//! The scoreboard (`/placar`) keeps objectives with a score per player: "manual" ones only
//! commands change, "mortes" counts deaths and "abates" players killed. One objective can be
//! shown at the side of the screen. Teams (`/equipe`) colour their members' names and, unless
//! told otherwise, can't hurt each other. Both live in `saves/<world>/scoreboard.ron` and the
//! host sends them to its guests whenever they change.

use glam::Vec3;
use net::protocol::{flags, Board, Objective, Team, ToGuest, ToHost};

use crate::audio::Sfx;
use crate::mobs::ray_box;
use crate::{level_path_hint, Game};

/// A hit only counts as a kill when the death follows it this soon (seconds).
const KILL_WINDOW: f32 = 15.0;
/// Players further apart than this can't hit each other (the host's check).
const MAX_REACH: f32 = 8.0;
const SIDEBAR_MAX: usize = 15;

/// Team colours: Portuguese name, English name, colour.
pub(crate) const COLORS: [(&str, &str, [u8; 4]); 16] = [
    ("branco", "white", [255, 255, 255, 255]),
    ("cinza", "gray", [170, 170, 170, 255]),
    ("cinza_escuro", "dark_gray", [110, 110, 110, 255]),
    ("preto", "black", [70, 70, 70, 255]),
    ("vermelho", "red", [255, 85, 85, 255]),
    ("vermelho_escuro", "dark_red", [190, 30, 30, 255]),
    ("laranja", "gold", [255, 170, 0, 255]),
    ("amarelo", "yellow", [255, 255, 85, 255]),
    ("verde", "green", [85, 255, 85, 255]),
    ("verde_escuro", "dark_green", [0, 170, 0, 255]),
    ("ciano", "aqua", [85, 255, 255, 255]),
    ("ciano_escuro", "dark_aqua", [0, 170, 170, 255]),
    ("azul", "blue", [85, 85, 255, 255]),
    ("azul_escuro", "dark_blue", [40, 40, 190, 255]),
    ("rosa", "light_purple", [255, 85, 255, 255]),
    ("roxo", "dark_purple", [170, 0, 170, 255]),
];

fn color_index(name: &str) -> Option<u8> {
    let n = name.to_lowercase();
    COLORS.iter().position(|(pt, en, _)| *pt == n || *en == n).map(|i| i as u8)
}

const KINDS: [&str; 3] = ["manual", "mortes", "abates"];

fn kind_of(s: &str) -> Option<&'static str> {
    match s.to_lowercase().as_str() {
        "manual" | "dummy" => Some("manual"),
        "mortes" | "deathcount" => Some("mortes"),
        "abates" | "playerkillcount" => Some("abates"),
        _ => None,
    }
}

pub(crate) fn board_path(base: &std::path::Path, world: &str) -> std::path::PathBuf {
    level_path_hint(base, world).join("scoreboard.ron")
}

pub(crate) fn load_board(base: &std::path::Path, world: &str) -> Board {
    std::fs::read_to_string(board_path(base, world)).ok().and_then(|t| ron::from_str(&t).ok()).unwrap_or_default()
}

impl Game {
    /// A swing at another player in front of us (nearer than the blocks and mobs there).
    /// True when it hit one.
    pub(crate) fn pvp_swing(&mut self, block_dist: f32) -> bool {
        if !self.net.active() || self.net.server {
            return false;
        }
        let eye = self.body.eye();
        let dir = self.forward();
        let reach = self.weapon_reach();
        let my_dim = self.dim.as_u8();
        let mob = self.mobs.pick(eye, dir, reach).map(|(_, t)| t).unwrap_or(f32::MAX);
        let mut best: Option<(u32, f32)> = None;
        for (id, r) in &self.net.remotes {
            let s = &r.state;
            if s.dim != my_dim || s.has(flags::DEAD) || s.has(flags::HIDDEN) {
                continue;
            }
            let tall = if s.crawling > 0.5 { 0.6 } else if s.sneaking > 0.5 { 1.5 } else { 1.8 };
            let lo = r.shown - Vec3::new(0.3, 0.0, 0.3);
            let hi = r.shown + Vec3::new(0.3, tall, 0.3);
            if let Some(t) = ray_box(eye, dir, lo, hi) {
                if t <= reach && t < block_dist && t < mob && best.is_none_or(|(_, b)| t < b) {
                    best = Some((*id, t));
                }
            }
        }
        let Some((target, _)) = best else { return false };
        let charge = if self.survival { self.attack_charge() } else { 1.0 };
        let base = if self.survival { self.surv_attack_damage() } else { 4.0 };
        // Falling onto someone with a ready weapon is a critical hit, as on mobs.
        let crit = self.body.vel.y < -0.5 && !self.body.on_ground && !self.body.in_water && charge > 0.9;
        let dmg = if crit { base * 1.5 } else { base };
        self.attack_cd = if self.survival { 0.1 } else { 0.45 };
        self.break_cd = 0.3;
        self.sfx(Sfx::MobHit);
        self.anim.start_swing();
        self.pads.rumble(0.3, 0.6, 0.1);
        if self.net.is_host() {
            let me = self.net_my_id();
            self.pvp_resolve(me, target, dmg, eye);
        } else {
            self.net_to_host(&ToHost::HurtPlayer { target, amount: dmg, from: eye.to_array() });
        }
        true
    }

    /// The host decides a hit of one player on another.
    pub(crate) fn pvp_resolve(&mut self, attacker: u32, target: u32, amount: f32, from: Vec3) {
        let me = self.net_my_id();
        if attacker == target || (target == me && self.net.server) {
            return;
        }
        if !self.net.rules.pvp {
            if attacker == me {
                self.toast("PvP desligado neste mundo (/regra pvp sim)", 3.0);
            } else if self.net.pvp_warned.insert(attacker) {
                self.net_to_guest(attacker, &ToGuest::Chat { from: String::new(), text: "PvP desligado neste mundo (/regra pvp sim)".into() });
            }
            return;
        }
        let (Some(a), Some(b)) = (self.net_pos_of(attacker), self.net_pos_of(target)) else { return };
        if (a - b).length() > MAX_REACH {
            return;
        }
        let (by, victim) = (self.net_name_of(attacker), self.net_name_of(target));
        if let (Some(ta), Some(tb)) = (self.net.board.team_of(&by), self.net.board.team_of(&victim)) {
            if ta.name == tb.name && !ta.friendly_fire {
                return;
            }
        }
        let amount = amount.clamp(0.0, 40.0);
        self.net.pvp_last.insert(victim.to_lowercase(), (by.clone(), self.now));
        if target == me {
            self.pvp_hurt_me(amount, from, &by);
        } else {
            self.net_to_guest(target, &ToGuest::PlayerHit { amount, from: from.to_array(), by });
        }
    }

    /// Another player hit us.
    pub(crate) fn pvp_hurt_me(&mut self, amount: f32, from: Vec3, by: &str) {
        self.net.pvp_last.insert(self.net.cfg.name.to_lowercase(), (by.to_string(), self.now));
        self.net.pvp_incoming = true;
        self.damage(amount, Some(from));
        self.net.pvp_incoming = false;
    }

    /// Someone died (the host's own player or a guest): the kill message and the scores.
    pub(crate) fn board_death(&mut self, name: &str) {
        if self.net.is_guest() {
            return;
        }
        let killer = self.net.pvp_last.remove(&name.to_lowercase()).filter(|(_, t)| self.now - *t < KILL_WINDOW).map(|(k, _)| k);
        let mut changed = self.net.board.count("mortes", name, 1);
        if let Some(k) = killer {
            changed |= self.net.board.count("abates", &k, 1);
            self.net_announce(format!("{name} foi derrotado por {k}"));
        }
        if changed {
            self.net.board_dirty = true;
        }
    }

    /// Every frame: our own death, and a changed board goes out to the guests and to disk.
    pub(crate) fn board_tick(&mut self) {
        let dead = self.screen == crate::Screen::Dead;
        if dead && !self.net.was_dead {
            let me = self.net.cfg.name.clone();
            self.board_death(&me);
        }
        self.net.was_dead = dead;
        self.pvp_test_hook();
        if self.net.board_dirty && !self.net.is_guest() {
            self.net.board_dirty = false;
            self.net_broadcast(&ToGuest::Board(self.net.board.clone()));
            if let Ok(t) = ron::ser::to_string_pretty(&self.net.board, Default::default()) {
                let _ = std::fs::write(board_path(&self.base, &self.world_name), t);
            }
        }
    }

    /// Test hook MC_PVP_HIT=n: once in, stand next to another player and hit them n times.
    fn pvp_test_hook(&mut self) {
        let Some(n) = std::env::var("MC_PVP_HIT").ok().and_then(|v| v.parse::<u32>().ok()) else { return };
        if self.screen != crate::Screen::Playing || !self.net.active() || self.net.pvp_test.0 >= n {
            return;
        }
        if self.now - self.net.pvp_test.1 < 1.0 {
            return;
        }
        let my_dim = self.dim.as_u8();
        let Some(p) = self.net.remotes.values().find(|r| r.state.dim == my_dim && !r.state.has(flags::HIDDEN)).map(|r| r.shown) else { return };
        self.net.pvp_test.1 = self.now;
        self.body.pos = p - Vec3::X * 2.0;
        self.body.vel = Vec3::ZERO;
        self.yaw = -std::f32::consts::FRAC_PI_2;
        self.pitch = 0.0;
        self.surv.attack_t = 10.0;
        if self.pvp_swing(f32::MAX) {
            self.net.pvp_test.0 += 1;
        }
    }

    /// The player a command is about: the one named, or whoever typed it.
    fn command_player(&self, arg: Option<&&str>) -> String {
        match arg {
            Some(n) if *n != "@s" && *n != "@p" => n.to_string(),
            _ if !self.net.cmd_by.is_empty() => self.net.cmd_by.clone(),
            _ => self.net.cfg.name.clone(),
        }
    }

    pub(crate) fn scoreboard_command(&mut self, args: &[&str]) -> String {
        const USE: &str = "Use: /placar novo <nome> [manual|mortes|abates] [titulo], mostrar <nome>|nada, somar|definir <jogador> <nome> <n>, zerar <jogador>, remover <nome>, lista";
        let sub = args.first().map(|s| s.to_lowercase()).unwrap_or_default();
        let b = &mut self.net.board;
        let out = match sub.as_str() {
            "novo" | "criar" | "add" => {
                let Some(name) = args.get(1) else { return USE.into() };
                if b.objectives.iter().any(|o| o.name.eq_ignore_ascii_case(name)) {
                    return format!("Já existe o placar {name}");
                }
                let kind = args.get(2).and_then(|k| kind_of(k));
                let title_from = if kind.is_some() { 3 } else { 2 };
                let title = if args.len() > title_from { args[title_from..].join(" ") } else { name.to_string() };
                b.objectives.push(Objective { name: name.to_string(), kind: kind.unwrap_or("manual").into(), title, scores: vec![] });
                format!("Placar {name} criado ({})", kind.unwrap_or("manual"))
            }
            "remover" | "remove" => {
                let Some(name) = args.get(1) else { return USE.into() };
                let before = b.objectives.len();
                b.objectives.retain(|o| !o.name.eq_ignore_ascii_case(name));
                if b.shown.as_deref().is_some_and(|s| s.eq_ignore_ascii_case(name)) {
                    b.shown = None;
                }
                if b.objectives.len() == before {
                    return format!("Placar desconhecido: {name}");
                }
                format!("Placar {name} removido")
            }
            "mostrar" | "setdisplay" => match args.get(1) {
                None => return USE.into(),
                Some(n) if ["nada", "nenhum", "none"].contains(&n.to_lowercase().as_str()) => {
                    b.shown = None;
                    "O placar saiu da tela".into()
                }
                Some(n) => {
                    let Some(o) = b.objective(n) else { return format!("Placar desconhecido: {n}") };
                    let name = o.name.clone();
                    b.shown = Some(name.clone());
                    format!("Mostrando o placar {name}")
                }
            },
            "somar" | "definir" | "tirar" => {
                let (Some(obj), Some(n)) = (args.get(2), args.get(3).and_then(|n| n.parse::<i32>().ok())) else { return USE.into() };
                let who = self.command_player(args.get(1));
                let b = &mut self.net.board;
                let Some(o) = b.objective(obj) else { return format!("Placar desconhecido: {obj}") };
                match sub.as_str() {
                    "somar" => o.add(&who, n),
                    "tirar" => o.add(&who, -n),
                    _ => o.set(&who, n),
                }
                format!("{who}: {} em {}", o.get(&who).unwrap_or(0), o.name)
            }
            "zerar" | "reset" => {
                let who = self.command_player(args.get(1));
                for o in self.net.board.objectives.iter_mut() {
                    o.scores.retain(|(p, _)| !p.eq_ignore_ascii_case(&who));
                }
                format!("Pontos de {who} zerados")
            }
            "lista" | "list" => {
                if b.objectives.is_empty() {
                    return "Nenhum placar (/placar novo <nome>)".into();
                }
                let l: Vec<String> = b.objectives.iter().map(|o| format!("{} ({}, {} jogadores)", o.name, o.kind, o.scores.len())).collect();
                return format!("Placares: {}", l.join(", "));
            }
            _ => return format!("{USE}. Tipos: {}", KINDS.join(", ")),
        };
        self.net.board_dirty = true;
        out
    }

    pub(crate) fn team_command(&mut self, args: &[&str]) -> String {
        const USE: &str = "Use: /equipe nova <nome> [cor], entrar <nome> [jogador], sair [jogador], cor <nome> <cor>, fogo_amigo <nome> sim|nao, remover <nome>, lista";
        let sub = args.first().map(|s| s.to_lowercase()).unwrap_or_default();
        let out = match sub.as_str() {
            "nova" | "criar" | "add" => {
                let Some(name) = args.get(1) else { return USE.into() };
                let b = &mut self.net.board;
                if b.teams.iter().any(|t| t.name.eq_ignore_ascii_case(name)) {
                    return format!("Já existe a equipe {name}");
                }
                let color = args.get(2).and_then(|c| color_index(c)).unwrap_or((b.teams.len() as u8 * 3 + 4) % 16);
                b.teams.push(Team { name: name.to_string(), color, friendly_fire: false, members: vec![] });
                format!("Equipe {name} criada ({})", COLORS[color as usize].0)
            }
            "remover" | "remove" => {
                let Some(name) = args.get(1) else { return USE.into() };
                let b = &mut self.net.board;
                let before = b.teams.len();
                b.teams.retain(|t| !t.name.eq_ignore_ascii_case(name));
                if b.teams.len() == before {
                    return format!("Equipe desconhecida: {name}");
                }
                format!("Equipe {name} removida")
            }
            "entrar" | "join" => {
                let Some(name) = args.get(1) else { return USE.into() };
                let who = self.command_player(args.get(2));
                let b = &mut self.net.board;
                if !b.teams.iter().any(|t| t.name.eq_ignore_ascii_case(name)) {
                    return format!("Equipe desconhecida: {name}");
                }
                for t in b.teams.iter_mut() {
                    t.members.retain(|m| !m.eq_ignore_ascii_case(&who));
                    if t.name.eq_ignore_ascii_case(name) {
                        t.members.push(who.clone());
                    }
                }
                format!("{who} entrou na equipe {name}")
            }
            "sair" | "leave" => {
                let who = self.command_player(args.get(1));
                for t in self.net.board.teams.iter_mut() {
                    t.members.retain(|m| !m.eq_ignore_ascii_case(&who));
                }
                format!("{who} saiu da equipe")
            }
            "cor" | "color" => {
                let (Some(name), Some(c)) = (args.get(1), args.get(2)) else { return USE.into() };
                let Some(ci) = color_index(c) else {
                    let names: Vec<&str> = COLORS.iter().map(|c| c.0).collect();
                    return format!("Cores: {}", names.join(", "));
                };
                let Some(t) = self.net.board.teams.iter_mut().find(|t| t.name.eq_ignore_ascii_case(name)) else { return format!("Equipe desconhecida: {name}") };
                t.color = ci;
                format!("Equipe {name}: {}", COLORS[ci as usize].0)
            }
            "fogo_amigo" | "friendlyfire" => {
                let (Some(name), Some(v)) = (args.get(1), args.get(2).and_then(|v| crate::commands::yes(v))) else { return USE.into() };
                let Some(t) = self.net.board.teams.iter_mut().find(|t| t.name.eq_ignore_ascii_case(name)) else { return format!("Equipe desconhecida: {name}") };
                t.friendly_fire = v;
                format!("Equipe {name}: fogo amigo {}", if v { "sim" } else { "não" })
            }
            "lista" | "list" => {
                let b = &self.net.board;
                if b.teams.is_empty() {
                    return "Nenhuma equipe (/equipe nova <nome>)".into();
                }
                let l: Vec<String> = b.teams.iter().map(|t| format!("{} [{}]: {}", t.name, COLORS[t.color as usize % 16].0, t.members.join(", "))).collect();
                return l.join("; ");
            }
            _ => return USE.into(),
        };
        self.net.board_dirty = true;
        out
    }

    /// A player's name colour: their team's, or white.
    pub(crate) fn team_color(&self, player: &str) -> [u8; 4] {
        self.net.board.team_of(player).map(|t| COLORS[t.color as usize % 16].2).unwrap_or([255, 255, 255, 255])
    }

    /// The shown objective at the right side of the screen, highest scores first.
    pub(crate) fn draw_scoreboard(&mut self, w: f32, h: f32) {
        let Some(shown) = self.net.board.shown.clone() else { return };
        let Some(o) = self.net.board.objectives.iter().find(|o| o.name == shown) else { return };
        let mut rows: Vec<(String, i32)> = o.scores.clone();
        rows.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.to_lowercase().cmp(&b.0.to_lowercase())));
        rows.truncate(SIDEBAR_MAX);
        let title = o.title.clone();
        let s = self.ui.scale;
        let lh = 9.0 * s;
        let lines: Vec<(String, String, [u8; 4])> = rows.iter().map(|(p, v)| (p.clone(), v.to_string(), self.team_color(p))).collect();
        let name_w = lines.iter().map(|(p, v, _)| self.ui.text_width(p, s) + self.ui.text_width(v, s) + 10.0 * s).fold(0.0, f32::max);
        let bw = name_w.max(self.ui.text_width(&title, s) + 8.0 * s);
        let bh = lh * (lines.len() as f32 + 1.0) + 2.0 * s;
        let x = w - bw - 2.0 * s;
        let y = (h - bh) / 2.0;
        self.ui.rect(x, y, bw, lh + s, [0, 0, 0, 110]);
        self.ui.rect(x, y + lh + s, bw, bh - lh - s, [0, 0, 0, 80]);
        self.ui.text_centered(x + bw / 2.0, y + s, &title, [255, 255, 255, 255]);
        for (i, (p, v, c)) in lines.iter().enumerate() {
            let ly = y + lh * (i as f32 + 1.0) + 2.0 * s;
            self.ui.text(x + 2.0 * s, ly, p, *c);
            let vw = self.ui.text_width(v, s);
            self.ui.text(x + bw - vw - 2.0 * s, ly, v, [255, 85, 85, 255]);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scores_and_teams() {
        let mut b = Board::default();
        b.objectives.push(Objective { name: "mortes".into(), kind: "mortes".into(), title: "Mortes".into(), scores: vec![] });
        assert!(b.count("mortes", "Ana", 1));
        assert!(b.count("mortes", "ana", 2));
        assert!(!b.count("abates", "Ana", 1));
        assert_eq!(b.objective("MORTES").unwrap().get("ANA"), Some(3));
        b.teams.push(Team { name: "azul".into(), color: 12, friendly_fire: false, members: vec!["Bia".into()] });
        assert_eq!(b.team_of("bia").map(|t| t.name.as_str()), Some("azul"));
        assert!(b.team_of("Ana").is_none());
        assert_eq!(color_index("dark_red"), Some(5));
        assert_eq!(color_index("azul"), Some(12));
    }
}
