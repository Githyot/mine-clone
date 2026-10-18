//! Captions for sounds (Lote 5 of plan/o-que-falta.md), an option: what made each sound nearby
//! and from which side, in the bottom right corner, fading after a few seconds.

use glam::Vec3;

use crate::audio::Sfx;
use crate::sfx_extra::Fx;
use crate::Game;

const SHOW: f32 = 3.0;
const MAX: usize = 6;

pub(crate) struct Subtitle {
    pub text: String,
    pub pos: Option<Vec3>,
    pub age: f32,
}

/// The caption of a sound, or None for the ones too common to caption (steps, music, rain).
pub(crate) fn caption(s: Sfx) -> Option<&'static str> {
    Some(match s {
        Sfx::Break(_) => "Bloco quebrado",
        Sfx::Place(_) => "Bloco colocado",
        Sfx::Hurt => "Jogador ferido",
        Sfx::Death => "Jogador morreu",
        Sfx::MobHit | Sfx::Hit(_) => "Golpe",
        Sfx::Explode => "Explosão",
        Sfx::Splash(_) => "Algo cai na água",
        Sfx::Pickup => "Item pego",
        Sfx::Xp => "Orbe de experiência",
        Sfx::Eat => "Comendo",
        Sfx::Fizz => "Chiado",
        Sfx::Thunder(_) => "Trovão",
        Sfx::Cast(_) => "Feitiço lançado",
        Sfx::SpellHit(_) => "Feitiço acerta",
        Sfx::Piston(_) => "Pistão",
        Sfx::Portal => "Portal zunindo",
        Sfx::Note(..) => "Bloco musical",
        Sfx::Fx(f) => match f {
            Fx::CaveDrone | Fx::CaveRumble => "Sons da caverna",
            Fx::Drip => "Gota caindo",
            Fx::LavaPop => "Lava borbulhando",
            Fx::FireCrackle => "Fogo crepitando",
            Fx::Waves => "Ondas",
            Fx::Heartbeat => "Coração batendo",
            Fx::Gasp => "Fôlego",
            Fx::Land(k) if k > 0.4 => "Queda",
            _ => return None,
        },
        _ => return None,
    })
}

impl Game {
    /// Note a sound for the captions (only while they are turned on).
    pub(crate) fn subtitle(&mut self, text: &str, pos: Option<Vec3>) {
        if !self.settings.subtitles {
            return;
        }
        if let Some(s) = self.subtitles.iter_mut().find(|s| s.text == text) {
            s.age = 0.0;
            s.pos = pos;
            return;
        }
        if self.subtitles.len() >= MAX {
            self.subtitles.remove(0);
        }
        self.subtitles.push(Subtitle { text: text.to_string(), pos, age: 0.0 });
    }

    pub(crate) fn update_subtitles(&mut self, dt: f32) {
        for s in self.subtitles.iter_mut() {
            s.age += dt;
        }
        self.subtitles.retain(|s| s.age < SHOW);
    }

    pub(crate) fn draw_subtitles(&mut self, w: f32, h: f32) {
        if !self.settings.subtitles || self.subtitles.is_empty() {
            return;
        }
        let s = self.ui.scale;
        let eye = self.body.eye();
        let f = self.forward();
        let right = Vec3::new(f.x, 0.0, f.z).cross(Vec3::Y).normalize_or_zero();
        let lines: Vec<(String, f32)> = self
            .subtitles
            .iter()
            .map(|t| {
                // An arrow on the side it came from; none when it is ahead or in the head.
                let side = t.pos.map(|p| (p - eye).normalize_or_zero().dot(right)).unwrap_or(0.0);
                let text = crate::lang::tr(&t.text).into_owned();
                let line = if side < -0.35 {
                    format!("< {text}")
                } else if side > 0.35 {
                    format!("{text} >")
                } else {
                    text
                };
                (line, 1.0 - (t.age / SHOW).powi(3))
            })
            .collect();
        let bw = lines.iter().map(|(l, _)| self.ui.text_width(l, s)).fold(0.0, f32::max) + 8.0 * s;
        let lh = 10.0 * s;
        let x = w - bw - 4.0 * s;
        let mut y = h - 40.0 * s - lines.len() as f32 * lh;
        for (l, a) in lines {
            let alpha = (a * 255.0) as u8;
            self.ui.rect(x, y - s, bw, lh, [0, 0, 0, (a * 150.0) as u8]);
            let tw = self.ui.text_width(&l, s);
            self.ui.text(x + (bw - tw) / 2.0, y, &l, [255, 255, 255, alpha]);
            y += lh;
        }
    }
}
