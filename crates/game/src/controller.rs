//! Game controllers.
//!
//! Sony controllers (DualShock 4 and DualSense) are read directly over HID, the way DS4Windows does
//! it, so the game gets the touchpad, the gyroscope, the light bar and rumble without any extra
//! program installed. Every other controller (Xbox, Switch Pro, generic) goes through gilrs.
//! If the HID device can't be opened (for example Linux without permission on /dev/hidraw*),
//! the Sony controller still works through gilrs, just without touchpad and light bar.

use gilrs::{Axis, Button, Gilrs};
use serde::{Deserialize, Serialize};

const SONY: u16 = 0x054C;
const DS4_PIDS: [u16; 4] = [0x05C4, 0x09CC, 0x0BA0, 0x0BA1];
const DS5_PIDS: [u16; 2] = [0x0CE6, 0x0DF2];

/// Controller buttons, named by position (South = Cross / A).
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Btn {
    South,
    East,
    West,
    North,
    L1,
    R1,
    L2,
    R2,
    Select,
    Start,
    L3,
    R3,
    Up,
    Down,
    Left,
    Right,
    Home,
    Touchpad,
}

impl Btn {
    pub const ALL: [Btn; 18] = [
        Btn::South,
        Btn::East,
        Btn::West,
        Btn::North,
        Btn::L1,
        Btn::R1,
        Btn::L2,
        Btn::R2,
        Btn::Select,
        Btn::Start,
        Btn::L3,
        Btn::R3,
        Btn::Up,
        Btn::Down,
        Btn::Left,
        Btn::Right,
        Btn::Home,
        Btn::Touchpad,
    ];

    fn bit(self) -> u32 {
        1 << (self as u32)
    }

    pub fn label(self, sony: bool) -> &'static str {
        match (self, sony) {
            (Btn::South, true) => "Xis",
            (Btn::East, true) => "Bola",
            (Btn::West, true) => "Quadrado",
            (Btn::North, true) => "Triângulo",
            (Btn::South, false) => "A",
            (Btn::East, false) => "B",
            (Btn::West, false) => "X",
            (Btn::North, false) => "Y",
            (Btn::L1, true) => "L1",
            (Btn::R1, true) => "R1",
            (Btn::L2, true) => "L2",
            (Btn::R2, true) => "R2",
            (Btn::L1, false) => "LB",
            (Btn::R1, false) => "RB",
            (Btn::L2, false) => "LT",
            (Btn::R2, false) => "RT",
            (Btn::Select, true) => "Share",
            (Btn::Start, true) => "Options",
            (Btn::Select, false) => "View",
            (Btn::Start, false) => "Menu",
            (Btn::L3, _) => "L3",
            (Btn::R3, _) => "R3",
            (Btn::Up, _) => "Seta cima",
            (Btn::Down, _) => "Seta baixo",
            (Btn::Left, _) => "Seta esquerda",
            (Btn::Right, _) => "Seta direita",
            (Btn::Home, true) => "PS",
            (Btn::Home, false) => "Guia",
            (Btn::Touchpad, _) => "Touchpad",
        }
    }
}

/// Things a button can do in the game.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Action {
    Jump,
    Sneak,
    Sprint,
    Attack,
    Use,
    PickBlock,
    Inventory,
    Pause,
    HotbarPrev,
    HotbarNext,
    Screenshot,
    Debug,
    NextSong,
    HideHud,
    Paintings,
    Crawl,
}

impl Action {
    pub const ALL: [Action; 16] = [
        Action::Jump,
        Action::Sneak,
        Action::Sprint,
        Action::Attack,
        Action::Use,
        Action::PickBlock,
        Action::Inventory,
        Action::Pause,
        Action::HotbarPrev,
        Action::HotbarNext,
        Action::Screenshot,
        Action::Debug,
        Action::NextSong,
        Action::HideHud,
        Action::Paintings,
        Action::Crawl,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Action::Jump => "Pular / voar (2x)",
            Action::Sneak => "Agachar / descer",
            Action::Sprint => "Correr",
            Action::Attack => "Quebrar / atacar",
            Action::Use => "Colocar / usar",
            Action::PickBlock => "Copiar bloco",
            Action::Inventory => "Inventário",
            Action::Pause => "Menu",
            Action::HotbarPrev => "Item anterior",
            Action::HotbarNext => "Próximo item",
            Action::Screenshot => "Foto",
            Action::Debug => "Informações (F3)",
            Action::NextSong => "Próxima música",
            Action::HideHud => "Esconder interface",
            Action::Paintings => "Pendurar pintura",
            Action::Crawl => "Deitar / rastejar",
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub enum TouchpadMode {
    /// Swipe left/right to change the selected item.
    Hotbar,
    /// The touchpad turns the camera, like a laptop touchpad (DS4Windows "mouse" mode).
    Camera,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(default)]
pub struct ControllerSettings {
    pub enabled: bool,
    /// 1..10
    pub look_sensitivity: u32,
    pub invert_y: bool,
    pub deadzone: f32,
    pub gyro_aim: bool,
    /// 1..10
    pub gyro_sensitivity: u32,
    pub touchpad_mode: TouchpadMode,
    pub rumble: bool,
    pub lightbar: bool,
    pub bindings: Vec<(Action, Btn)>,
}

impl Default for ControllerSettings {
    fn default() -> Self {
        ControllerSettings {
            enabled: true,
            look_sensitivity: 5,
            invert_y: false,
            deadzone: 0.12,
            gyro_aim: false,
            gyro_sensitivity: 5,
            touchpad_mode: TouchpadMode::Hotbar,
            rumble: true,
            lightbar: true,
            bindings: default_bindings(),
        }
    }
}

pub fn default_bindings() -> Vec<(Action, Btn)> {
    vec![
        (Action::Jump, Btn::South),
        (Action::Sneak, Btn::East),
        (Action::Sprint, Btn::L3),
        (Action::Attack, Btn::R2),
        (Action::Use, Btn::L2),
        (Action::PickBlock, Btn::West),
        (Action::Inventory, Btn::North),
        (Action::Pause, Btn::Start),
        (Action::HotbarPrev, Btn::L1),
        (Action::HotbarNext, Btn::R1),
        (Action::Screenshot, Btn::Select),
        (Action::Debug, Btn::Up),
        (Action::NextSong, Btn::Right),
        (Action::HideHud, Btn::Down),
        (Action::Paintings, Btn::Left),
        (Action::Crawl, Btn::R3),
    ]
}

impl ControllerSettings {
    /// Actions added after the settings were saved get their default button.
    pub fn fill_missing(&mut self) {
        for (a, b) in default_bindings() {
            if self.button_for(a).is_none() && !self.bindings.iter().any(|(_, x)| *x == b) {
                self.bindings.push((a, b));
            }
        }
    }

    pub fn button_for(&self, a: Action) -> Option<Btn> {
        self.bindings.iter().find(|(x, _)| *x == a).map(|(_, b)| *b)
    }

    pub fn bind(&mut self, a: Action, b: Btn) {
        self.bindings.retain(|(x, _)| *x != a);
        self.bindings.push((a, b));
    }
}

/// The current state of the active controller.
#[derive(Clone, Default)]
pub struct Pad {
    pub connected: bool,
    pub name: String,
    pub sony: bool,
    /// Read over HID (touchpad, gyro and light bar available).
    pub hid: bool,
    buttons: u32,
    prev: u32,
    pub lx: f32,
    pub ly: f32,
    pub rx: f32,
    pub ry: f32,
    pub l2: f32,
    pub r2: f32,
    /// Fingers on the touchpad, 0..1 in both axes (y down).
    pub touch: [Option<[f32; 2]>; 2],
    pub prev_touch: [Option<[f32; 2]>; 2],
    /// Angular velocity in degrees per second: pitch, yaw, roll.
    pub gyro: [f32; 3],
    pub battery: Option<u8>,
}

impl Pad {
    pub fn down(&self, b: Btn) -> bool {
        self.buttons & b.bit() != 0
    }
    pub fn pressed(&self, b: Btn) -> bool {
        self.buttons & b.bit() != 0 && self.prev & b.bit() == 0
    }
    /// First button that went down this frame (for remapping).
    pub fn any_pressed(&self) -> Option<Btn> {
        Btn::ALL.iter().copied().find(|&b| self.pressed(b))
    }
    pub fn touch_delta(&self) -> Option<[f32; 2]> {
        match (self.touch[0], self.prev_touch[0]) {
            (Some(a), Some(b)) if self.touch[1].is_none() => Some([a[0] - b[0], a[1] - b[1]]),
            _ => None,
        }
    }
}

struct SonyDevice {
    dev: hidapi::HidDevice,
    ds5: bool,
    bluetooth: bool,
    sent: ([u8; 3], u8, u8),
    resend: f32,
}

pub struct Controllers {
    gilrs: Option<Gilrs>,
    api: Option<hidapi::HidApi>,
    sony: Option<SonyDevice>,
    scan_timer: f32,
    pub pad: Pad,
    rumble_strong: f32,
    rumble_weak: f32,
    rumble_left: f32,
    pub light: [u8; 3],
}

fn axis(v: u8) -> f32 {
    ((v as f32 - 128.0) / 127.0).clamp(-1.0, 1.0)
}

fn crc32(seed: &[u8], data: &[u8]) -> u32 {
    let mut crc = 0xFFFF_FFFFu32;
    for &b in seed.iter().chain(data) {
        crc ^= b as u32;
        for _ in 0..8 {
            crc = if crc & 1 != 0 { (crc >> 1) ^ 0xEDB8_8320 } else { crc >> 1 };
        }
    }
    !crc
}

impl Controllers {
    pub fn new() -> Controllers {
        let gilrs = Gilrs::new().map_err(|e| log::warn!("gamepads unavailable: {e}")).ok();
        let api = hidapi::HidApi::new().map_err(|e| log::warn!("HID unavailable: {e}")).ok();
        Controllers {
            gilrs,
            api,
            sony: None,
            scan_timer: 0.0,
            pad: Pad::default(),
            rumble_strong: 0.0,
            rumble_weak: 0.0,
            rumble_left: 0.0,
            light: [0, 0, 64],
        }
    }

    /// Shake the controller. `strong` is the heavy motor, `weak` the light one (0..1).
    pub fn rumble(&mut self, strong: f32, weak: f32, secs: f32) {
        self.rumble_strong = self.rumble_strong.max(strong);
        self.rumble_weak = self.rumble_weak.max(weak);
        self.rumble_left = self.rumble_left.max(secs);
    }

    fn scan_sony(&mut self) {
        let Some(api) = self.api.as_mut() else { return };
        if api.refresh_devices().is_err() {
            return;
        }
        let found = api.device_list().find(|d| {
            d.vendor_id() == SONY && (DS4_PIDS.contains(&d.product_id()) || DS5_PIDS.contains(&d.product_id()))
        });
        let Some(info) = found else { return };
        let ds5 = DS5_PIDS.contains(&info.product_id());
        let bluetooth = matches!(info.bus_type(), hidapi::BusType::Bluetooth);
        match info.open_device(api) {
            Ok(dev) => {
                let _ = dev.set_blocking_mode(false);
                // Over Bluetooth the controller only sends short reports until a feature report is read.
                let mut feat = [0u8; 64];
                feat[0] = if ds5 { 0x05 } else { 0x02 };
                let _ = dev.get_feature_report(&mut feat);
                if !ds5 {
                    feat[0] = 0x05;
                    let _ = dev.get_feature_report(&mut feat);
                }
                log::info!("{} connected over {}", if ds5 { "DualSense" } else { "DualShock 4" }, if bluetooth { "Bluetooth" } else { "USB" });
                self.sony = Some(SonyDevice { dev, ds5, bluetooth, sent: ([0; 3], 255, 255), resend: 0.0 });
            }
            Err(e) => log::info!("Sony controller found but HID could not be opened ({e}); using the generic driver"),
        }
    }

    pub fn update(&mut self, dt: f32, settings: &ControllerSettings) {
        self.pad.prev = self.pad.buttons;
        self.pad.prev_touch = self.pad.touch;
        if !settings.enabled {
            self.pad.connected = false;
            self.pad.buttons = 0;
            return;
        }
        self.scan_timer -= dt;
        if self.sony.is_none() && self.scan_timer <= 0.0 {
            self.scan_timer = 2.0;
            self.scan_sony();
        }

        let mut got_sony = false;
        if let Some(s) = self.sony.as_mut() {
            let mut buf = [0u8; 128];
            let mut last: Option<(usize, [u8; 128])> = None;
            let mut lost = false;
            for _ in 0..64 {
                match s.dev.read(&mut buf) {
                    Ok(0) => break,
                    Ok(n) => last = Some((n, buf)),
                    Err(_) => {
                        lost = true;
                        break;
                    }
                }
            }
            if lost {
                log::info!("Sony controller disconnected");
                self.sony = None;
                self.pad = Pad::default();
            } else {
                got_sony = true;
                if let Some((n, b)) = last {
                    parse_sony(&mut self.pad, &b[..n], s.ds5);
                }
                self.pad.connected = true;
                self.pad.hid = true;
                self.pad.sony = true;
                self.pad.name = if s.ds5 { "DualSense".into() } else { "DualShock 4".into() };
            }
        }

        if let Some(g) = self.gilrs.as_mut() {
            while g.next_event().is_some() {}
            if !got_sony {
                let pick = g.gamepads().find(|(_, gp)| gp.is_connected()).map(|(id, _)| id);
                match pick {
                    Some(id) => {
                        let gp = g.gamepad(id);
                        let p = &mut self.pad;
                        p.connected = true;
                        p.hid = false;
                        p.sony = gp.vendor_id() == Some(SONY);
                        p.name = gp.name().to_string();
                        let map = [
                            (Button::South, Btn::South),
                            (Button::East, Btn::East),
                            (Button::West, Btn::West),
                            (Button::North, Btn::North),
                            (Button::LeftTrigger, Btn::L1),
                            (Button::RightTrigger, Btn::R1),
                            (Button::Select, Btn::Select),
                            (Button::Start, Btn::Start),
                            (Button::LeftThumb, Btn::L3),
                            (Button::RightThumb, Btn::R3),
                            (Button::DPadUp, Btn::Up),
                            (Button::DPadDown, Btn::Down),
                            (Button::DPadLeft, Btn::Left),
                            (Button::DPadRight, Btn::Right),
                            (Button::Mode, Btn::Home),
                        ];
                        let mut bits = 0;
                        for (gb, b) in map {
                            if gp.is_pressed(gb) {
                                bits |= b.bit();
                            }
                        }
                        p.l2 = gp.button_data(Button::LeftTrigger2).map(|d| d.value()).unwrap_or(0.0);
                        p.r2 = gp.button_data(Button::RightTrigger2).map(|d| d.value()).unwrap_or(0.0);
                        if p.l2 > 0.4 {
                            bits |= Btn::L2.bit();
                        }
                        if p.r2 > 0.4 {
                            bits |= Btn::R2.bit();
                        }
                        p.buttons = bits;
                        p.lx = gp.value(Axis::LeftStickX);
                        p.ly = gp.value(Axis::LeftStickY);
                        p.rx = gp.value(Axis::RightStickX);
                        p.ry = gp.value(Axis::RightStickY);
                        p.touch = [None, None];
                        p.gyro = [0.0; 3];
                    }
                    None => self.pad = Pad { prev: self.pad.buttons, ..Pad::default() },
                }
            }
        }

        // Dead zone (radial).
        let dz = settings.deadzone.clamp(0.0, 0.6);
        for (x, y) in [(&mut self.pad.lx, &mut self.pad.ly), (&mut self.pad.rx, &mut self.pad.ry)] {
            let m = (*x * *x + *y * *y).sqrt();
            if m < dz {
                *x = 0.0;
                *y = 0.0;
            } else {
                let k = ((m - dz) / (1.0 - dz)).min(1.0) / m;
                *x *= k;
                *y *= k;
            }
        }

        // Rumble and light bar.
        self.rumble_left -= dt;
        if self.rumble_left <= 0.0 {
            self.rumble_strong = 0.0;
            self.rumble_weak = 0.0;
        }
        if let Some(s) = self.sony.as_mut() {
            let strong = if settings.rumble { (self.rumble_strong * 255.0) as u8 } else { 0 };
            let weak = if settings.rumble { (self.rumble_weak * 255.0) as u8 } else { 0 };
            let light = if settings.lightbar { self.light } else { [0, 0, 0] };
            s.resend -= dt;
            let state = (light, strong, weak);
            if state != s.sent || s.resend <= 0.0 {
                s.sent = state;
                s.resend = 1.0;
                let _ = s.dev.write(&output_report(s.ds5, s.bluetooth, light, strong, weak));
            }
        }
    }

}

/// Fill `pad` from a DualShock 4 or DualSense input report.
fn parse_sony(pad: &mut Pad, r: &[u8], ds5: bool) {
    if r.is_empty() {
        return;
    }
    // b = index of the left stick X byte.
    let (b, full) = match (ds5, r[0]) {
        (false, 0x01) => (1, r.len() >= 64),
        (false, 0x11) => (3, r.len() >= 78),
        (true, 0x01) => (1, r.len() >= 64),
        (true, 0x31) => (2, r.len() >= 78),
        _ => return,
    };
    if r.len() < b + 10 {
        return;
    }
    pad.lx = axis(r[b]);
    pad.ly = -axis(r[b + 1]);
    pad.rx = axis(r[b + 2]);
    pad.ry = -axis(r[b + 3]);
    let (btn, l2, r2) = if ds5 { (b + 7, r[b + 4], r[b + 5]) } else { (b + 4, r[b + 7], r[b + 8]) };
    let (b0, b1, b2) = (r[btn], r[btn + 1], r[btn + 2]);
    let mut bits = 0u32;
    let dpad = b0 & 0x0F;
    let dirs = [(0, Btn::Up), (1, Btn::Up), (7, Btn::Up), (1, Btn::Right), (2, Btn::Right), (3, Btn::Right), (3, Btn::Down), (4, Btn::Down), (5, Btn::Down), (5, Btn::Left), (6, Btn::Left), (7, Btn::Left)];
    for (d, btn) in dirs {
        if dpad == d {
            bits |= btn.bit();
        }
    }
    let flags = [
        (b0, 4, Btn::West),
        (b0, 5, Btn::South),
        (b0, 6, Btn::East),
        (b0, 7, Btn::North),
        (b1, 0, Btn::L1),
        (b1, 1, Btn::R1),
        (b1, 4, Btn::Select),
        (b1, 5, Btn::Start),
        (b1, 6, Btn::L3),
        (b1, 7, Btn::R3),
        (b2, 0, Btn::Home),
        (b2, 1, Btn::Touchpad),
    ];
    for (byte, bit, btn) in flags {
        if byte & (1 << bit) != 0 {
            bits |= btn.bit();
        }
    }
    pad.l2 = l2 as f32 / 255.0;
    pad.r2 = r2 as f32 / 255.0;
    if pad.l2 > 0.4 {
        bits |= Btn::L2.bit();
    }
    if pad.r2 > 0.4 {
        bits |= Btn::R2.bit();
    }
    pad.buttons = bits;
    if !full {
        return;
    }
    let i16at = |i: usize| i16::from_le_bytes([r[i], r[i + 1]]) as f32;
    let gyro = if ds5 { b + 15 } else { b + 12 };
    // Both controllers report about 16.4 units per degree per second.
    pad.gyro = [i16at(gyro) / 16.4, i16at(gyro + 2) / 16.4, i16at(gyro + 4) / 16.4];
    let touch = if ds5 { b + 32 } else { b + 34 };
    let height = if ds5 { 1080.0 } else { 942.0 };
    for k in 0..2 {
        let t = touch + k * 4;
        if r.len() < t + 4 {
            break;
        }
        let active = r[t] & 0x80 == 0;
        let x = (r[t + 1] as u32 | ((r[t + 2] as u32 & 0x0F) << 8)) as f32;
        let y = ((r[t + 2] as u32 >> 4) | ((r[t + 3] as u32) << 4)) as f32;
        pad.touch[k] = active.then(|| [(x / 1920.0).clamp(0.0, 1.0), (y / height).clamp(0.0, 1.0)]);
    }
    let status = if ds5 { b + 52 } else { b + 29 };
    if r.len() > status {
        pad.battery = Some(((r[status] & 0x0F) * 10).min(100));
    }
}

fn output_report(ds5: bool, bluetooth: bool, light: [u8; 3], strong: u8, weak: u8) -> Vec<u8> {
    if !ds5 {
        if bluetooth {
            let mut r = vec![0u8; 78];
            r[0] = 0x11;
            r[1] = 0xC0;
            r[2] = 0x20;
            r[3] = 0xF3;
            r[4] = 0x04;
            r[6] = weak;
            r[7] = strong;
            r[8] = light[0];
            r[9] = light[1];
            r[10] = light[2];
            let crc = crc32(&[0xA2], &r[..74]);
            r[74..78].copy_from_slice(&crc.to_le_bytes());
            r
        } else {
            let mut r = vec![0u8; 32];
            r[0] = 0x05;
            r[1] = 0x07;
            r[2] = 0x04;
            r[4] = weak;
            r[5] = strong;
            r[6] = light[0];
            r[7] = light[1];
            r[8] = light[2];
            r
        }
    } else {
        // DualSense: same payload, one byte further in over Bluetooth, with a checksum.
        let (len, off) = if bluetooth { (78, 2) } else { (48, 1) };
        let mut r = vec![0u8; len];
        if bluetooth {
            r[0] = 0x31;
            r[1] = 0x02;
        } else {
            r[0] = 0x02;
        }
        r[off] = 0x03; // rumble
        r[off + 1] = 0x04; // light bar
        r[off + 2] = weak;
        r[off + 3] = strong;
        r[off + 38] = 0x02;
        r[off + 41] = 0x02;
        r[off + 44] = light[0];
        r[off + 45] = light[1];
        r[off + 46] = light[2];
        if bluetooth {
            let crc = crc32(&[0xA2], &r[..74]);
            r[74..78].copy_from_slice(&crc.to_le_bytes());
        }
        r
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_ds4_usb_report() {
        let mut r = [0u8; 64];
        r[0] = 0x01;
        r[1] = 255; // left stick right
        r[2] = 0; // left stick up
        r[3] = 128;
        r[4] = 128;
        r[5] = 0x08 | (1 << 5); // d-pad released, Cross
        r[6] = 1 << 1; // R1
        r[7] = 1 << 1; // touchpad click
        r[9] = 255; // R2
        // One finger in the middle of the touchpad.
        r[35] = 0x01;
        let (x, y) = (960u32, 471u32);
        r[36] = (x & 0xFF) as u8;
        r[37] = ((x >> 8) as u8 & 0x0F) | (((y & 0x0F) as u8) << 4);
        r[38] = (y >> 4) as u8;
        r[39] = 0x80; // second finger up
        let mut p = Pad::default();
        parse_sony(&mut p, &r, false);
        assert!(p.down(Btn::South) && p.down(Btn::R1) && p.down(Btn::Touchpad) && p.down(Btn::R2));
        assert!(!p.down(Btn::Up) && !p.down(Btn::East));
        assert!((p.lx - 1.0).abs() < 0.01 && (p.ly - 1.0).abs() < 0.02);
        let t = p.touch[0].unwrap();
        assert!((t[0] - 0.5).abs() < 0.01 && (t[1] - 0.5).abs() < 0.01);
        assert!(p.touch[1].is_none());
    }

    #[test]
    fn bluetooth_report_has_valid_crc() {
        let r = output_report(false, true, [255, 0, 0], 10, 20);
        let crc = crc32(&[0xA2], &r[..74]);
        assert_eq!(&r[74..78], &crc.to_le_bytes());
        // Known value: CRC-32 of "123456789" is CBF43926.
        assert_eq!(crc32(&[], b"123456789"), 0xCBF4_3926);
    }
}
