//! Who a command is for (Lote 9): `@p` the nearest player, `@a` everyone, `@r` someone at
//! random, `@s` whoever runs it (for a command block, the nearest player) and `@e` everyone
//! plus, for `/matar`, the creatures around. A player's name works too: `/dar Ana diamond 3`,
//! `/tp @p 0 80 0`, `/modo criativo @a`. A command about a player runs on that player's
//! machine; the host passes it on to a guest.
//!
//! Also the world-editing commands command blocks are made for: `/setblock x y z block`,
//! `/fill x1 y1 z1 x2 y2 z2 block` and `/summon mob [x y z]`, where `~` is relative to the
//! command block (or to the player who typed it).

use glam::{IVec3, Vec3};
use voxel::AIR;

use crate::commands::{scope, Scope};
use crate::Game;

/// A player a command can name: None is this machine's own.
pub(crate) struct Target {
    pub id: Option<u32>,
    pub name: String,
    pub pos: Vec3,
}

const MAX_FILL: i64 = 32 * 32 * 32;

impl Game {
    /// Everyone playing in this dimension (a dedicated server's own player is nobody).
    pub(crate) fn cmd_targets(&self) -> Vec<Target> {
        let mut out = Vec::new();
        if !self.net.server {
            out.push(Target { id: None, name: self.net.cfg.name.clone(), pos: self.body.pos });
        }
        let dim = self.dim.as_u8();
        for (id, r) in &self.net.remotes {
            if r.state.dim == dim && !r.state.has(net::protocol::flags::HIDDEN) {
                out.push(Target { id: Some(*id), name: r.name.clone(), pos: Vec3::from(r.state.pos) });
            }
        }
        out
    }

    /// The command once per player its selector picks (`@a` gives one line each). Err when
    /// it picks nobody.
    fn expand_selectors(&mut self, line: &str, origin: Vec3) -> Result<Vec<String>, String> {
        let words: Vec<&str> = line.split_whitespace().collect();
        let Some(k) = words.iter().position(|w| w.starts_with('@')) else { return Ok(vec![line.to_string()]) };
        // "@p[distance=..10]": the options are not used.
        let sel = words[k].split('[').next().unwrap_or("").to_lowercase();
        let mut all = self.cmd_targets();
        if all.is_empty() {
            return Err("Nenhum jogador".into());
        }
        all.sort_by(|a, b| a.pos.distance_squared(origin).total_cmp(&b.pos.distance_squared(origin)));
        let picked: Vec<String> = match sel.as_str() {
            "@p" | "@s" => vec![all[0].name.clone()],
            "@r" => {
                let i = (self.surv.rng.f32() * all.len() as f32) as usize;
                vec![all[i.min(all.len() - 1)].name.clone()]
            }
            "@a" | "@e" => all.iter().map(|t| t.name.clone()).collect(),
            _ => return Err(format!("Seletor desconhecido: {} (use @p, @a, @r, @s ou @e)", words[k])),
        };
        let mut out = Vec::new();
        for name in picked {
            let mut w: Vec<String> = words.iter().map(|s| s.to_string()).collect();
            w[k] = name;
            // More selectors in the same line pick on their own.
            out.extend(self.expand_selectors(&w.join(" "), origin)?);
        }
        Ok(out)
    }

    /// Run a command that may be aimed at a player (by name or selector), from `origin`.
    /// Returns what to show.
    pub(crate) fn run_command_at(&mut self, line: &str, origin: Vec3) -> String {
        let cmd = line.split_whitespace().next().unwrap_or("").to_lowercase();
        let mut said = Vec::new();
        if matches!(cmd.as_str(), "/matar" | "/kill") && line.split_whitespace().any(|w| w.to_lowercase().starts_with("@e")) {
            said.push(self.kill_creatures(origin));
        }
        let lines = match self.expand_selectors(line, origin) {
            Ok(l) => l,
            Err(e) => return e,
        };
        for l in lines {
            let r = self.run_command_on(&l, origin);
            if !r.is_empty() {
                said.push(r);
            }
        }
        said.join(" | ")
    }

    fn run_command_on(&mut self, line: &str, origin: Vec3) -> String {
        let words: Vec<&str> = line.split_whitespace().collect();
        let cmd = words.first().map(|c| c.to_lowercase()).unwrap_or_default();
        // /tp is anyone's to use on themselves, and the host's to use on others.
        if scope(&cmd) != Scope::Player && cmd != "/tp" {
            return self.run_command(line, true);
        }
        // A player named in it (anywhere after the command) is who it is for.
        let targets = self.cmd_targets();
        let named = words.iter().enumerate().skip(1).find_map(|(i, w)| targets.iter().position(|t| t.name.eq_ignore_ascii_case(w)).map(|t| (i, t)));
        let (rest, t) = match named {
            Some((i, t)) => {
                let rest: Vec<&str> = words.iter().enumerate().filter(|&(j, _)| j != i).map(|(_, w)| *w).collect();
                (rest.join(" "), &targets[t])
            }
            None => {
                // Nobody named: the nearest player to where it came from.
                let Some(t) = targets.iter().min_by(|a, b| a.pos.distance_squared(origin).total_cmp(&b.pos.distance_squared(origin))) else { return "Nenhum jogador".into() };
                (line.to_string(), t)
            }
        };
        match t.id {
            None => self.run_command(&rest, true),
            Some(id) if self.net.is_host() => {
                self.net_to_guest(id, &net::protocol::ToGuest::RunAs(rest.clone()));
                format!("{}: {rest}", t.name)
            }
            Some(_) => "Só quem hospeda manda comandos para outro jogador".into(),
        }
    }

    /// `/matar @e`: the creatures within 64 blocks, but not pets, town folk or named ones.
    fn kill_creatures(&mut self, origin: Vec3) -> String {
        let mut n = 0;
        for i in 0..self.mobs.list.len() {
            let m = &self.mobs.list[i];
            let def = &self.mobs.reg.defs[m.def];
            let keep = def.town || m.name.is_some() || m.leashed || m.saddled || def.name.ends_with("_tame") || matches!(def.name.as_str(), "armor_stand" | "snow_golem" | "allay");
            if m.alive() && !keep && m.body.pos.distance(origin) < 64.0 {
                self.mobs.list[i].health = 0.0;
                n += 1;
            }
        }
        format!("{n} criaturas eliminadas")
    }

    /// A coordinate: a number, or `~`/`~n` relative to `base`.
    fn coord(s: &str, base: i32) -> Option<i32> {
        if let Some(r) = s.strip_prefix('~') {
            return Some(base + if r.is_empty() { 0 } else { r.parse::<f32>().ok()?.floor() as i32 });
        }
        Some(s.parse::<f32>().ok()?.floor() as i32)
    }

    fn coords(args: &[&str], base: IVec3) -> Option<IVec3> {
        Some(IVec3::new(Self::coord(args.first()?, base.x)?, Self::coord(args.get(1)?, base.y)?, Self::coord(args.get(2)?, base.z)?))
    }

    /// Where `~` counts from: the command block running, or the player.
    fn cmd_base(&self) -> IVec3 {
        self.cmdblocks.origin.unwrap_or_else(|| self.body.pos.floor().as_ivec3())
    }

    /// `/setblock`, `/fill`, `/summon` (and their Portuguese names). None: not one of them.
    pub(crate) fn edit_command(&mut self, cmd: &str, args: &[&str]) -> Option<String> {
        let base = self.cmd_base();
        let block = |g: &Game, n: &str| {
            let n = n.trim_start_matches("minecraft:");
            if n == "air" || n == "ar" {
                Some(AIR)
            } else {
                g.reg.id(n)
            }
        };
        Some(match cmd {
            "/setblock" | "/bloco" => {
                let (Some(p), Some(n)) = (Self::coords(args, base), args.get(3)) else { return Some("Use: /setblock <x> <y> <z> <bloco>".into()) };
                let Some(id) = block(self, n) else { return Some(format!("Bloco desconhecido: {n}")) };
                if !self.world.set_block(p, id) {
                    return Some("Esse lugar não está carregado".into());
                }
                format!("Bloco posto em {} {} {}", p.x, p.y, p.z)
            }
            "/fill" | "/preencher" => {
                let (Some(a), Some(b), Some(n)) = (Self::coords(args, base), Self::coords(args.get(3..).unwrap_or(&[]), base), args.get(6)) else {
                    return Some("Use: /fill <x1> <y1> <z1> <x2> <y2> <z2> <bloco>".into());
                };
                let Some(id) = block(self, n) else { return Some(format!("Bloco desconhecido: {n}")) };
                let (lo, hi) = (a.min(b), a.max(b));
                let size = (hi - lo + IVec3::ONE).as_i64vec3();
                if size.x * size.y * size.z > MAX_FILL {
                    return Some(format!("Grande demais: no máximo {MAX_FILL} blocos"));
                }
                let mut n = 0;
                for y in lo.y..=hi.y {
                    for z in lo.z..=hi.z {
                        for x in lo.x..=hi.x {
                            if self.world.set_block(IVec3::new(x, y, z), id) {
                                n += 1;
                            }
                        }
                    }
                }
                format!("{n} blocos preenchidos")
            }
            "/summon" | "/invocar" => {
                let Some(name) = args.first() else { return Some("Use: /summon <criatura> [x y z]".into()) };
                let name = name.trim_start_matches("minecraft:");
                let p = Self::coords(&args[1..], base).unwrap_or(base);
                let at = p.as_vec3() + Vec3::new(0.5, 0.0, 0.5);
                if self.mobs.reg.id(name).is_none() || !self.mobs.spawn(name, at) {
                    return Some(format!("Criatura desconhecida: {name}"));
                }
                format!("{} apareceu", self.mobs.reg.defs[self.mobs.reg.id(name).unwrap()].display_name)
            }
            _ => return None,
        })
    }
}
