//! The event bars at the top of the screen (goblin invasion, wave invasions, jelly rain, the
//! pillars) and, in multijogador, the host's events shown to the guests in its world (lote 18a):
//! the host runs the events, so a guest sees the host's bars and its blood moon or eclipse.

use net::{EventBar, EventsFx, ToGuest};

use crate::Game;

/// The host sends the events this often even if nothing changed (a guest that just came in).
const EVERY: f32 = 2.0;

#[derive(Default)]
pub struct NetEvents {
    t: f32,
    last: EventsFx,
    /// Guest: what the host last said.
    pub(crate) from_host: Option<EventsFx>,
}

fn bar(title: &str, frac: f32, color: [u8; 3]) -> EventBar {
    EventBar { title: title.to_string(), frac: frac.clamp(0.0, 1.0), color }
}

impl Game {
    /// The events this machine runs, as bars.
    pub(crate) fn events_here(&self) -> EventsFx {
        let mut bars = Vec::new();
        if let Some(k) = self.invasion_progress() {
            bars.push(bar("Invasão Goblin", k, [200, 140, 60]));
        }
        if let Some((title, k)) = self.invasion_hud() {
            bars.push(bar(&title, k, [230, 150, 60]));
        }
        if let Some((killed, need)) = self.t5.tr.ev.rain_progress() {
            bars.push(bar("Chuva de Geleias", killed as f32 / need.max(1) as f32, [80, 150, 230]));
        }
        let line = self.pillar_status().map(|l| format!("Pilares: {l}")).unwrap_or_default();
        EventsFx { bars, line, blood: self.t5.hard.blood_moon(), eclipse: self.t5.tr.ev.eclipse }
    }

    /// The events to show: the host's when it runs our world, else our own.
    fn events_shown(&self) -> EventsFx {
        match self.net_events.from_host.as_ref() {
            Some(e) if !self.net_simulates() => e.clone(),
            _ => self.events_here(),
        }
    }

    /// Every frame. Host: tell the guests in its world when the events change. Guest in the
    /// host's world: its sky follows the host's.
    pub(crate) fn update_net_events(&mut self, dt: f32) {
        if self.net.is_guest() {
            if self.net_simulates() {
                return;
            }
            if let Some(e) = self.net_events.from_host.as_ref() {
                let (blood, eclipse) = (e.blood, e.eclipse);
                self.t5.hard.blood = blood;
                self.t5.tr.ev.eclipse = eclipse;
            }
            return;
        }
        if !self.net.is_host() {
            self.net_events = NetEvents::default();
            return;
        }
        self.net_events.t += dt;
        let now = self.events_here();
        let changed = now.bars.len() != self.net_events.last.bars.len()
            || now.line != self.net_events.last.line
            || now.blood != self.net_events.last.blood
            || now.eclipse != self.net_events.last.eclipse
            || now.bars.iter().zip(&self.net_events.last.bars).any(|(a, b)| a.title != b.title || (a.frac - b.frac).abs() > 0.004);
        if !changed && self.net_events.t < EVERY {
            return;
        }
        self.net_events.t = 0.0;
        let dim = self.dim.as_u8();
        for (id, gdim, _) in self.net_guest_spots() {
            // Guests elsewhere run their own world's events.
            if gdim == dim {
                self.net_to_guest(id, &ToGuest::Events(now.clone()));
            }
        }
        self.net_events.last = now;
    }

    /// The bars, stacked under the boss bar if there is one.
    pub(crate) fn draw_event_bars(&mut self, w: f32) {
        let ev = self.events_shown();
        let s = self.ui.scale;
        let boss_up = self.mobs.list.iter().any(|m| m.alive() && self.mobs.reg.defs[m.def].boss && m.body.pos.distance(self.body.pos) < 64.0);
        let mut y = if boss_up { 38.0 * s } else { 14.0 * s };
        let bw = 150.0 * s;
        let x = (w - bw) / 2.0;
        for b in &ev.bars {
            let [r, g, bl] = b.color;
            let tw = self.ui.text_width(&b.title, s);
            self.ui.text((w - tw) / 2.0, y - 10.0 * s, &b.title, [240, 215, 170, 255]);
            self.ui.rect(x - s, y - s, bw + 2.0 * s, 6.0 * s, [0, 0, 0, 200]);
            self.ui.rect(x, y, bw, 4.0 * s, [r / 4, g / 4, bl / 4, 255]);
            self.ui.rect(x, y, bw * b.frac, 4.0 * s, [r, g, bl, 255]);
            y += 24.0 * s;
        }
        if !ev.line.is_empty() {
            let tw = self.ui.text_width(&ev.line, s);
            self.ui.text((w - tw) / 2.0, y - 10.0 * s, &ev.line, [200, 210, 255, 255]);
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn events_cross_the_wire() {
        let e = net::EventsFx { bars: vec![super::bar("Invasão Goblin", 1.5, [1, 2, 3])], line: "Pilares: Solar 20".into(), blood: true, eclipse: false };
        assert_eq!(e.bars[0].frac, 1.0);
        let m = net::ToGuest::Events(e);
        assert_eq!(net::decode::<net::ToGuest>(&net::encode(&m)), Some(m));
    }
}
