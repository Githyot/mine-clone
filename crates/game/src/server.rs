//! Dedicated server (Lote 7 of plan/o-que-falta.md): `mine-clone --server [--world NAME]`.
//!
//! The same game with its window hidden and nothing drawn: it opens the world, hosts it for
//! friends (port, password and UPnP from `multiplayer.ron`), and keeps simulating about 20
//! times a second. Its own player is nobody: hidden from the others, not listed, never hurt,
//! parked high above the spawn so the spawn area stays loaded. Lines typed in the terminal
//! are commands (`/op Ana`, `/tempo dia`, `/regra pvp sim`) or chat; `parar` saves and stops.
//!
//! It still needs a graphics device (wgpu) because the client and the world share one
//! program; on a machine without a GPU, a software Vulkan driver (lavapipe/Mesa) and a
//! virtual display (xvfb-run on Linux) are enough.

use std::io::BufRead;
use std::time::{Duration, Instant};

use glam::Vec3;

use crate::{Game, Screen};

const TICK: Duration = Duration::from_millis(50);

impl Game {
    /// One server step. False when it was told to stop.
    pub(crate) fn server_tick(&mut self) -> bool {
        if !self.net.server {
            self.server_start();
        }
        let now = Instant::now();
        let since = now - self.last;
        if since < TICK {
            std::thread::sleep(TICK - since);
        }
        let now = Instant::now();
        let dt = (now - self.last).as_secs_f32().min(0.1);
        self.last = now;
        self.update(dt);
        if self.screen == Screen::Dead || self.screen == Screen::Paused {
            self.set_screen(Screen::Playing);
        }
        if self.world_open && self.screen == Screen::Playing {
            // Parked above the spawn, out of everyone's way.
            let spawn = self.surv.spawn.map(|p| p.as_vec3()).unwrap_or(Vec3::new(self.spawn_xz.0 as f32, 0.0, self.spawn_xz.1 as f32));
            self.body.flying = true;
            self.body.vel = Vec3::ZERO;
            self.body.pos = Vec3::new(spawn.x + 0.5, 300.0, spawn.z + 0.5);
            self.health = self.health.max(1.0);
        }
        let lines: Vec<String> = self.net.console.as_ref().map(|rx| rx.try_iter().collect()).unwrap_or_default();
        for line in lines {
            let line = line.trim().to_string();
            if line.is_empty() {
                continue;
            }
            match line.to_lowercase().as_str() {
                "parar" | "/parar" | "stop" | "/stop" | "sair" => {
                    log::info!("saving and stopping");
                    self.save_all();
                    self.net_stop();
                    return false;
                }
                "ajuda" | "help" => log::info!("{} | parar: salva e desliga", crate::commands::HELP),
                _ => self.server_line(&line),
            }
        }
        true
    }

    fn server_start(&mut self) {
        self.net_test_hooks();
        self.net.server = true;
        self.net.cfg.name = "Servidor".into();
        self.net.host_when_ready = true;
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            for line in std::io::stdin().lock().lines().map_while(Result::ok) {
                if tx.send(line).is_err() {
                    break;
                }
            }
        });
        self.net.console = Some(rx);
        log::info!("dedicated server: opening \"{}\"; type commands here (ajuda, parar)", self.world_name);
    }
}
