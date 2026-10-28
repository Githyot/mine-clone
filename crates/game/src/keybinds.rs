//! Keyboard keys the player can move (Lote 5 of plan/o-que-falta.md). The game reads its
//! default keys everywhere; a key moved in the options is turned back into that default as it
//! arrives from the window, so nothing else needs to know about it.

use std::collections::HashMap;

use winit::keyboard::KeyCode;

use crate::{Game, Screen};

/// (setting name, label, default key)
pub(crate) const ACTIONS: [(&str, &str, KeyCode); 23] = [
    ("forward", "Andar para frente", KeyCode::KeyW),
    ("back", "Andar para tras", KeyCode::KeyS),
    ("left", "Andar para a esquerda", KeyCode::KeyA),
    ("right", "Andar para a direita", KeyCode::KeyD),
    ("jump", "Pular", KeyCode::Space),
    ("sneak", "Agachar", KeyCode::ShiftLeft),
    ("sprint", "Correr", KeyCode::ControlLeft),
    ("crawl", "Deitar", KeyCode::KeyZ),
    ("inventory", "Inventário", KeyCode::KeyE),
    ("drop", "Soltar item", KeyCode::KeyQ),
    ("chat", "Conversa", KeyCode::KeyT),
    ("perspective", "Mudar a câmera", KeyCode::F5),
    ("hide_hud", "Esconder a interface", KeyCode::F1),
    ("screenshot", "Foto da tela", KeyCode::F2),
    ("debug", "Informações (F3)", KeyCode::F3),
    ("paintings", "Quadros", KeyCode::KeyP),
    ("music", "Próxima música", KeyCode::KeyM),
    ("glyphs", "Grimório", KeyCode::KeyG),
    ("skills", "Ofícios", KeyCode::KeyY),
    ("hook", "Gancho", KeyCode::KeyF),
    ("emote", "Emotes", KeyCode::KeyI),
    ("voice_talk", "Falar (chat de voz)", KeyCode::CapsLock),
    ("voice_mute", "Microfone liga/desliga", KeyCode::KeyK),
];

/// Every key that can be bound, so a name in settings.ron turns back into a key.
const KEYS: [KeyCode; 76] = [
    KeyCode::KeyA, KeyCode::KeyB, KeyCode::KeyC, KeyCode::KeyD, KeyCode::KeyE, KeyCode::KeyF, KeyCode::KeyG,
    KeyCode::KeyH, KeyCode::KeyI, KeyCode::KeyJ, KeyCode::KeyK, KeyCode::KeyL, KeyCode::KeyM, KeyCode::KeyN,
    KeyCode::KeyO, KeyCode::KeyP, KeyCode::KeyQ, KeyCode::KeyR, KeyCode::KeyS, KeyCode::KeyT, KeyCode::KeyU,
    KeyCode::KeyV, KeyCode::KeyW, KeyCode::KeyX, KeyCode::KeyY, KeyCode::KeyZ, KeyCode::Digit0, KeyCode::Digit1,
    KeyCode::Digit2, KeyCode::Digit3, KeyCode::Digit4, KeyCode::Digit5, KeyCode::Digit6, KeyCode::Digit7,
    KeyCode::Digit8, KeyCode::Digit9, KeyCode::F1, KeyCode::F2, KeyCode::F3, KeyCode::F4, KeyCode::F5, KeyCode::F6,
    KeyCode::F7, KeyCode::F8, KeyCode::F9, KeyCode::F10, KeyCode::F11, KeyCode::F12, KeyCode::Space, KeyCode::Tab,
    KeyCode::CapsLock, KeyCode::ShiftLeft, KeyCode::ShiftRight, KeyCode::ControlLeft, KeyCode::ControlRight,
    KeyCode::AltLeft, KeyCode::AltRight, KeyCode::ArrowUp, KeyCode::ArrowDown, KeyCode::ArrowLeft,
    KeyCode::ArrowRight, KeyCode::Comma, KeyCode::Period, KeyCode::Slash, KeyCode::Semicolon, KeyCode::Quote,
    KeyCode::BracketLeft, KeyCode::BracketRight, KeyCode::Backslash, KeyCode::Minus, KeyCode::Equal,
    KeyCode::Backquote, KeyCode::Numpad0, KeyCode::Numpad1, KeyCode::Numpad2, KeyCode::Numpad3,
];

fn key_name(k: KeyCode) -> String {
    format!("{k:?}")
}

fn key_from(name: &str) -> Option<KeyCode> {
    KEYS.iter().copied().find(|&k| key_name(k) == name)
}

/// How a key shows on its button: "W", "5", "Espaço", "Shift esq.".
pub(crate) fn key_label(k: KeyCode) -> String {
    let n = key_name(k);
    match k {
        KeyCode::Space => "Espaço".into(),
        KeyCode::ShiftLeft => "Shift esq.".into(),
        KeyCode::ShiftRight => "Shift dir.".into(),
        KeyCode::ControlLeft => "Ctrl esq.".into(),
        KeyCode::ControlRight => "Ctrl dir.".into(),
        KeyCode::AltLeft => "Alt esq.".into(),
        KeyCode::AltRight => "Alt dir.".into(),
        KeyCode::ArrowUp => "Seta acima".into(),
        KeyCode::ArrowDown => "Seta abaixo".into(),
        KeyCode::ArrowLeft => "Seta esq.".into(),
        KeyCode::ArrowRight => "Seta dir.".into(),
        _ => n.strip_prefix("Key").or_else(|| n.strip_prefix("Digit")).unwrap_or(&n).to_string(),
    }
}

/// Physical key → the default key the game reads, built from the player's bindings.
#[derive(Default)]
pub(crate) struct KeyMap {
    map: HashMap<KeyCode, Option<KeyCode>>,
}

impl KeyMap {
    pub(crate) fn new(binds: &std::collections::BTreeMap<String, String>) -> KeyMap {
        let mut map = HashMap::new();
        let mut taken = vec![];
        for (id, _, default) in ACTIONS {
            let Some(k) = binds.get(id).and_then(|n| key_from(n)) else { continue };
            if k != default {
                map.insert(k, Some(default));
                taken.push(default);
            }
        }
        // A default key given away to nothing else stops doing its old job.
        for d in taken {
            map.entry(d).or_insert(None);
        }
        KeyMap { map }
    }

    pub(crate) fn get(&self, k: KeyCode) -> Option<KeyCode> {
        self.map.get(&k).copied().unwrap_or(Some(k))
    }
}

impl Game {
    pub(crate) fn bound_key(&self, i: usize) -> KeyCode {
        let (id, _, default) = ACTIONS[i];
        self.settings.key_binds.get(id).and_then(|n| key_from(n)).unwrap_or(default)
    }

    /// A key from the window: the rebinding screen takes it raw, the game gets its default.
    pub(crate) fn window_key(&mut self, code: KeyCode, pressed: bool, repeat: bool) {
        if self.screen == Screen::Keys {
            if let Some(i) = self.key_rebinding {
                if pressed && !repeat {
                    if code != KeyCode::Escape {
                        let (id, _, default) = ACTIONS[i];
                        if code == default {
                            self.settings.key_binds.remove(id);
                        } else {
                            self.settings.key_binds.insert(id.to_string(), key_name(code));
                        }
                        self.key_map = KeyMap::new(&self.settings.key_binds);
                        self.settings.save(&self.base.join("settings.ron"));
                    }
                    self.key_rebinding = None;
                }
                return;
            }
        }
        if let Some(c) = self.key_map.get(code) {
            self.key(c, pressed, repeat);
        }
    }

    pub(crate) fn draw_keys(&mut self, w: f32, _h: f32, mouse: (f32, f32)) {
        let s = self.ui.scale;
        self.ui.text_centered(w / 2.0, 8.0 * s, "Teclas", [255, 255, 255, 255]);
        let bw = 150.0 * s;
        let bh = 18.0 * s;
        let gap = 3.0 * s;
        let rows = ACTIONS.len().div_ceil(2);
        let y0 = 22.0 * s;
        let mut hit = None;
        for (i, action) in ACTIONS.iter().enumerate() {
            let x = w / 2.0 - bw - gap / 2.0 + (i % 2) as f32 * (bw + gap);
            let y = y0 + (i / 2) as f32 * (bh + gap);
            let key = if self.key_rebinding == Some(i) { "> ? <".to_string() } else { key_label(self.bound_key(i)) };
            let moved = self.bound_key(i) != action.2;
            let label = format!("{}: {}{}", action.1, key, if moved { " *" } else { "" });
            if self.ui.button(x, y, bw, bh, &label, mouse) && self.clicked {
                hit = Some(i);
            }
        }
        let y = y0 + rows as f32 * (bh + gap) + gap;
        let reset = self.ui.button(w / 2.0 - bw - gap / 2.0, y, bw, 20.0 * s, "Restaurar padrão", mouse) && self.clicked;
        let done = self.ui.button(w / 2.0 + gap / 2.0, y, bw, 20.0 * s, "Concluído", mouse) && self.clicked;
        if self.key_rebinding.is_some() {
            self.ui.text_centered(w / 2.0, y + 26.0 * s, "Aperte a nova tecla (Esc cancela)", [255, 255, 160, 255]);
        }
        if let Some(i) = hit {
            self.sfx(crate::audio::Sfx::Click);
            self.key_rebinding = Some(i);
        } else if reset {
            self.sfx(crate::audio::Sfx::Click);
            self.settings.key_binds.clear();
            self.key_map = KeyMap::new(&self.settings.key_binds);
            self.settings.save(&self.base.join("settings.ron"));
        } else if done {
            self.sfx(crate::audio::Sfx::Click);
            self.key_rebinding = None;
            self.set_screen(Screen::Options);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn moved_keys_turn_into_their_defaults() {
        let mut binds = std::collections::BTreeMap::new();
        binds.insert("forward".to_string(), "ArrowUp".to_string());
        binds.insert("jump".to_string(), "KeyW".to_string());
        let m = KeyMap::new(&binds);
        assert_eq!(m.get(KeyCode::ArrowUp), Some(KeyCode::KeyW));
        assert_eq!(m.get(KeyCode::KeyW), Some(KeyCode::Space));
        assert_eq!(m.get(KeyCode::Space), None);
        assert_eq!(m.get(KeyCode::KeyE), Some(KeyCode::KeyE));
        for (_, _, k) in ACTIONS {
            assert!(key_from(&key_name(k)).is_some(), "{k:?}");
        }
    }
}
