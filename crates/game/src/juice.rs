//! Interface "juice" (Immersive UI idea, our own code): the hotbar selector slides between
//! slots, the item under the mouse grows a little, the stack on the cursor sways as it is moved,
//! slots holding the same item as the cursor bob so you can see where it stacks, and rare items
//! leave a short trail of sparks. All of it is a few numbers per frame (no extra passes), and the
//! "Animações da interface" option turns it off (reduce motion).

use std::time::Instant;

use crate::survival::Slot;
use crate::ui::UiBatch;

const TRAIL_MAX: usize = 48;

struct Spark {
    x: f32,
    y: f32,
    vx: f32,
    vy: f32,
    life: f32,
    color: [u8; 3],
}

pub struct UiJuice {
    last: Instant,
    pub dt: f32,
    /// Hotbar selector position in slots (eases toward the selected slot).
    sel: f32,
    /// Slot under the mouse and how far its grow animation is (0..1).
    hover: Option<(Slot, f32)>,
    /// Cursor stack sway: angle and its speed (a damped spring pushed by the mouse).
    pub angle: f32,
    spin: f32,
    mouse: Option<(f32, f32)>,
    sparks: Vec<Spark>,
    seed: u32,
    pub time: f32,
}

impl Default for UiJuice {
    fn default() -> Self {
        UiJuice { last: Instant::now(), dt: 0.0, sel: 0.0, hover: None, angle: 0.0, spin: 0.0, mouse: None, sparks: Vec::new(), seed: 0x51f1_5eed, time: 0.0 }
    }
}

/// Rarity colour of an item by its name, like the classic tooltip colours: yellow uncommon,
/// aqua rare, light purple epic. None for common items.
pub fn rarity(name: &str) -> Option<[u8; 3]> {
    const EPIC: [&str; 7] = ["netherite", "nether_star", "dragon", "elytra", "totem", "enchanted", "phoenix"];
    const RARE: [&str; 9] = ["diamond", "emerald", "golden_apple", "trident", "zanite", "gravitite", "valkyrie", "resonarium", "heart"];
    const UNCOMMON: [&str; 6] = ["gold", "ruby", "sapphire", "topaz", "platinum", "amethyst"];
    if EPIC.iter().any(|k| name.contains(k)) {
        Some([255, 85, 255])
    } else if RARE.iter().any(|k| name.contains(k)) {
        Some([85, 255, 255])
    } else if UNCOMMON.iter().any(|k| name.contains(k)) {
        Some([255, 255, 85])
    } else {
        None
    }
}

/// Whether the animations run: the option, and not in test screenshots (which show things at
/// rest) unless MC_UI_ANIM is set.
pub fn enabled(setting: bool, screenshot: bool) -> bool {
    static FORCE: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    setting && (!screenshot || *FORCE.get_or_init(|| std::env::var("MC_UI_ANIM").is_ok()))
}

impl UiJuice {
    fn rand(&mut self) -> f32 {
        self.seed ^= self.seed << 13;
        self.seed ^= self.seed >> 17;
        self.seed ^= self.seed << 5;
        self.seed as f32 / u32::MAX as f32
    }

    /// Once per frame, before drawing the interface.
    pub fn tick(&mut self, selected: usize, on: bool) {
        let now = Instant::now();
        self.dt = (now - self.last).as_secs_f32().min(0.1);
        self.last = now;
        self.time += self.dt;
        let target = selected as f32;
        // A jump across most of the bar (scrolling past the end) snaps instead of sliding back.
        if !on || (target - self.sel).abs() > 4.5 {
            self.sel = target;
        } else {
            self.sel += (target - self.sel) * (1.0 - (-self.dt * 22.0).exp());
        }
        for s in &mut self.sparks {
            s.x += s.vx * self.dt;
            s.y += s.vy * self.dt;
            s.vy += 30.0 * self.dt;
            s.life -= self.dt;
        }
        self.sparks.retain(|s| s.life > 0.0);
        if !on {
            self.sparks.clear();
            self.angle = 0.0;
            self.spin = 0.0;
            self.hover = None;
        }
    }

    /// Hotbar selector x, in slots.
    pub fn selector(&self) -> f32 {
        self.sel
    }

    /// Size factor for the icon in `slot` (the hovered one grows).
    pub fn slot_scale(&mut self, slot: Slot, hovered: bool, on: bool) -> f32 {
        if !on {
            return 1.0;
        }
        match (&mut self.hover, hovered) {
            (Some((s, t)), true) if *s == slot => {
                *t = (*t + self.dt * 9.0).min(1.0);
            }
            (_, true) => self.hover = Some((slot, 0.0)),
            _ => {}
        }
        match self.hover {
            Some((s, t)) if s == slot && hovered => {
                let e = 1.0 - (1.0 - t) * (1.0 - t);
                1.0 + 0.2 * e
            }
            _ => 1.0,
        }
    }

    /// Vertical bob (pixels × scale) for a slot holding what the cursor holds.
    pub fn match_bob(&self, index: usize, on: bool) -> f32 {
        if on {
            ((self.time * 7.0 + index as f32 * 0.6).sin() * 0.5 + 0.5) * -2.0
        } else {
            0.0
        }
    }

    /// Move the cursor stack: sway from the mouse's speed, sparks for rare items.
    pub fn cursor(&mut self, mouse: (f32, f32), rare: Option<[u8; 3]>, scale: f32, on: bool) {
        let prev = self.mouse.replace(mouse);
        if !on || self.dt <= 0.0 {
            return;
        }
        let (vx, vy) = prev.map(|p| ((mouse.0 - p.0) / self.dt, (mouse.1 - p.1) / self.dt)).unwrap_or((0.0, 0.0));
        // Spring toward a lean against the motion; damped so it settles in a swing or two.
        let push = (-vx / scale * 0.0009).clamp(-0.6, 0.6);
        self.spin += (push - self.angle) * 140.0 * self.dt - self.spin * 11.0 * self.dt;
        self.angle = (self.angle + self.spin * self.dt).clamp(-0.7, 0.7);
        if let Some(c) = rare {
            let speed = (vx * vx + vy * vy).sqrt();
            let n = ((speed / scale * self.dt * 0.08) as usize).min(3) + usize::from(self.rand() < self.dt * 6.0);
            for _ in 0..n {
                if self.sparks.len() >= TRAIL_MAX {
                    self.sparks.remove(0);
                }
                let (jx, jy) = (self.rand() - 0.5, self.rand() - 0.5);
                let s = Spark { x: mouse.0 + jx * 10.0 * scale, y: mouse.1 + jy * 10.0 * scale, vx: jx * 20.0 * scale, vy: -8.0 * scale, life: 0.35 + self.rand() * 0.35, color: c };
                self.sparks.push(s);
            }
        }
    }

    /// Draw the spark trail (under the cursor stack).
    pub fn draw_sparks(&self, ui: &mut UiBatch) {
        let s = ui.scale;
        for p in &self.sparks {
            let a = (p.life / 0.5).clamp(0.0, 1.0);
            let size = s * if p.life > 0.3 { 1.5 } else { 1.0 };
            ui.rect(p.x.floor(), p.y.floor(), size, size, [p.color[0], p.color[1], p.color[2], (a * 230.0) as u8]);
        }
    }
}
