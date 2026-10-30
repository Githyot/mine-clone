//! Command blocks (Lote 7 of plan/o-que-falta.md): a block that runs a chat command each time
//! redstone power reaches it. Only someone allowed to use commands, in creative, can set its
//! command (right click opens the editor); it is kept per world in `command_blocks.ron`.
//! A guest's edit goes to the host, where the world and its redstone run.
//!
//! The command runs on the host with every permission. Commands about a player (/dar, /tp,
//! /modo) act on the nearest one, or on whom `@p`, `@a`, `@r`, `@s`, `@e` or a name picks
//! (selectors.rs); `~` in /setblock, /fill and /summon counts from the block.

use std::collections::HashMap;
use std::path::Path;

use glam::IVec3;

use crate::{Game, Screen};

/// The chat line a guest sends to set a block's command: `/definir_bloco <dim> x y z <command>`.
pub const SET_CMD: &str = "/definir_bloco";
const MAX_LEN: usize = 200;

#[derive(Default)]
pub(crate) struct CmdBlocks {
    /// "dim x y z" → command.
    pub texts: HashMap<String, String>,
    /// The block being edited and the text so far.
    pub edit: Option<(IVec3, String)>,
    /// The block whose command is running (where `~` counts from).
    pub origin: Option<IVec3>,
}

fn key(dim: u8, p: IVec3) -> String {
    format!("{dim} {} {} {}", p.x, p.y, p.z)
}

pub(crate) fn load(dir: &Path) -> CmdBlocks {
    let texts = std::fs::read_to_string(dir.join("command_blocks.ron")).ok().and_then(|t| ron::from_str(&t).ok()).unwrap_or_default();
    CmdBlocks { texts, edit: None, origin: None }
}

pub(crate) fn save(dir: &Path, c: &CmdBlocks) {
    if c.texts.is_empty() && !dir.join("command_blocks.ron").exists() {
        return;
    }
    let _ = std::fs::create_dir_all(dir);
    if let Ok(t) = ron::ser::to_string_pretty(&c.texts, Default::default()) {
        let _ = voxel::save::write_atomic(dir.join("command_blocks.ron"), t);
    }
}

impl Game {
    /// May this player set commands? Creative, and alone, hosting or made operator.
    fn may_command(&self) -> bool {
        !self.survival && self.net_cheats()
    }

    /// Right click on a command block.
    pub(crate) fn cmdblock_open(&mut self, p: IVec3) -> bool {
        if !self.may_command() {
            self.toast("Só quem pode usar comandos, no modo criativo, mexe no bloco de comando", 3.0);
            return true;
        }
        let text = self.cmdblocks.texts.get(&key(self.dim.as_u8(), p)).cloned().unwrap_or_default();
        self.cmdblocks.edit = Some((p, text));
        self.set_screen(Screen::Sign);
        true
    }

    pub(crate) fn cmdblock_key(&mut self, code: winit::keyboard::KeyCode) {
        use winit::keyboard::KeyCode;
        let Some((_, t)) = self.cmdblocks.edit.as_mut() else { return };
        match code {
            KeyCode::Backspace => {
                t.pop();
            }
            KeyCode::Enter | KeyCode::NumpadEnter => self.cmdblock_close(true),
            KeyCode::Escape => self.cmdblock_close(false),
            _ => {}
        }
    }

    pub(crate) fn cmdblock_type(&mut self, text: &str) {
        let Some((_, t)) = self.cmdblocks.edit.as_mut() else { return };
        for c in text.chars() {
            if !c.is_control() && t.chars().count() < MAX_LEN {
                t.push(c);
            }
        }
    }

    fn cmdblock_close(&mut self, done: bool) {
        if let Some((p, text)) = self.cmdblocks.edit.take() {
            if done {
                let text = text.trim().to_string();
                let dim = self.dim.as_u8();
                self.cmdblock_set(dim, p, &text);
                if self.net.is_guest() {
                    let line = format!("{SET_CMD} {dim} {} {} {} {text}", p.x, p.y, p.z);
                    self.net_to_host(&net::protocol::ToHost::Chat(line));
                }
                self.toast(if text.is_empty() { "Bloco de comando vazio".to_string() } else { format!("Comando: {text}") }, 2.5);
            }
        }
        self.set_screen(Screen::Playing);
    }

    fn cmdblock_set(&mut self, dim: u8, p: IVec3, text: &str) {
        let k = key(dim, p);
        if text.is_empty() {
            self.cmdblocks.texts.remove(&k);
        } else {
            self.cmdblocks.texts.insert(k, text.to_string());
        }
    }

    /// The host got a guest's `/definir_bloco` line.
    pub(crate) fn cmdblock_set_line(&mut self, line: &str) -> String {
        let mut parts = line.splitn(6, ' ');
        let _ = parts.next();
        let nums: Vec<i32> = parts.by_ref().take(4).filter_map(|n| n.parse().ok()).collect();
        let text = parts.next().unwrap_or("").trim().to_string();
        if nums.len() != 4 {
            return String::new();
        }
        self.cmdblock_set(nums[0] as u8, IVec3::new(nums[1], nums[2], nums[3]), &text);
        "Bloco de comando definido".into()
    }

    /// Power reached a command block (redstone, on its rising edge): run its command.
    pub(crate) fn cmdblock_run(&mut self, p: IVec3) {
        let Some(cmd) = self.cmdblocks.texts.get(&key(self.dim.as_u8(), p)).cloned() else { return };
        let cmd = if cmd.starts_with('/') { cmd } else { format!("/{cmd}") };
        self.cmdblocks.origin = Some(p);
        let out = self.run_command_at(&cmd, p.as_vec3() + glam::Vec3::splat(0.5));
        self.cmdblocks.origin = None;
        // The world changed for everyone: the sky and scores go out now.
        self.net_sky_now();
        if !out.is_empty() && (p.as_vec3() - self.body.pos).length() < 32.0 {
            self.toast(format!("[Bloco de comando] {out}"), 3.0);
        }
    }

    /// The editor: the command line, wide, with a cursor.
    pub(crate) fn draw_cmdblock_editor(&mut self, w: f32, h: f32) {
        let s = self.ui.scale;
        let text = self.cmdblocks.edit.as_ref().map(|e| e.1.clone()).unwrap_or_default();
        let bw = (w - 40.0 * s).min(360.0 * s);
        let bh = 20.0 * s;
        let (x, y) = ((w - bw) / 2.0, (h - bh) / 2.0);
        self.ui.rect(0.0, 0.0, w, h, [0, 0, 0, 140]);
        self.ui.text_centered(w / 2.0, y - 22.0 * s, "Bloco de comando", [255, 255, 255, 255]);
        self.ui.rect(x - s, y - s, bw + 2.0 * s, bh + 2.0 * s, [160, 160, 160, 255]);
        self.ui.rect(x, y, bw, bh, [0, 0, 0, 255]);
        let blink = (self.now * 2.0) as i64 % 2 == 0;
        // The end of a long command stays in view.
        let mut shown: String = text.clone();
        while self.ui.text_width(&shown, s) > bw - 12.0 * s && !shown.is_empty() {
            shown.remove(0);
        }
        let shown = if blink { format!("{shown}_") } else { shown };
        self.ui.text(x + 4.0 * s, y + 6.0 * s, &shown, [255, 255, 255, 255]);
        self.ui.text_centered(w / 2.0, y + bh + 10.0 * s, "Um comando de chat (ex.: /tempo dia). Roda quando recebe energia.", [200, 200, 200, 255]);
        self.ui.text_centered(w / 2.0, y + bh + 22.0 * s, "Enter: pronto. Esc: cancela", [255, 255, 255, 255]);
    }
}
