//! Lote 11 of the Terraria side (plan/terraria-completo.md): looks and sound.
//!
//! - Vanity clothes: eight outfits (hat, coat, trousers, boots). Right click puts a piece on
//!   over the armour (only the look changes; the armour still protects). The Vanity Mirror
//!   takes them all off again.
//! - Paint: the brush recolours one block, the roller a 3x3 patch, with the first dye in the
//!   hotbar (one dye paints 8 blocks): wool, carpets, concrete, its powder, terracotta, glass
//!   and panes. The scraper takes the colour off.
//! - The music box: right click it while a song plays to record it; after that, right click
//!   plays it (on repeat) or stops it.

use std::collections::HashMap;

use glam::{IVec3, Vec3};
use voxel::item::ItemStack;
use voxel::physics::Terrain;

use crate::audio::{Material, Sfx};
use crate::Game;

const COLOURS: [&str; 16] = ["white", "orange", "magenta", "light_blue", "yellow", "lime", "pink", "gray", "light_gray", "cyan", "purple", "blue", "brown", "green", "red", "black"];
/// Families that come in the 16 colours, and what the scraper leaves of them.
const FAMILIES: [(&str, &str); 7] = [
    ("_wool", "white_wool"),
    ("_carpet", "white_carpet"),
    ("_concrete_powder", "white_concrete_powder"),
    ("_concrete", "white_concrete"),
    ("_stained_glass_pane", "glass_pane"),
    ("_stained_glass", "glass"),
    ("_terracotta", "terracotta"),
];
const RANGE: f32 = 48.0;

#[derive(Default)]
pub struct Style {
    pub vanity: [Option<ItemStack>; 4],
    /// Blocks painted since the last dye was used up.
    paint_uses: u8,
    /// Music boxes and the song each one recorded.
    pub boxes: HashMap<IVec3, String>,
    pub playing: Option<IVec3>,
}

fn piece_of(name: &str) -> Option<usize> {
    let rest = name.strip_prefix("vanity_")?;
    ["_helmet", "_chestplate", "_leggings", "_boots"].iter().position(|s| rest.ends_with(s))
}

/// The family (suffix) of a coloured or plain block, or None.
fn family(name: &str) -> Option<usize> {
    for (k, (suffix, plain)) in FAMILIES.iter().enumerate() {
        if name == *plain || COLOURS.iter().any(|c| name.strip_prefix(c).is_some_and(|r| r == *suffix)) {
            return Some(k);
        }
    }
    None
}

impl Game {
    pub(crate) fn tr_style_use(&mut self, name: &str) -> Option<bool> {
        if let Some(piece) = piece_of(name) {
            let sel = self.selected;
            let held = self.surv.inv.slots[sel].take();
            let old = std::mem::replace(&mut self.t5.tr.style.vanity[piece], held);
            self.surv.inv.slots[sel] = old;
            self.sfx(Sfx::Place(Material::Cloth));
            self.toast("Roupa de vaidade vestida (por cima da armadura)", 2.0);
            return Some(true);
        }
        match name {
            "vanity_mirror" => {
                let mut any = false;
                for k in 0..4 {
                    if let Some(st) = self.t5.tr.style.vanity[k].take() {
                        any = true;
                        if let Some(left) = self.surv.inv.add(&self.items, st) {
                            let at = self.body.pos + Vec3::Y;
                            self.spawn_drop(at, left, Vec3::Y);
                        }
                    }
                }
                self.toast(if any { "Roupas de vaidade tiradas" } else { "Você não está usando roupas de vaidade" }, 2.0);
                Some(true)
            }
            "paint_brush" => Some(self.paint(false, false)),
            "paint_roller" => Some(self.paint(true, false)),
            "paint_scraper" => Some(self.paint(false, true)),
            _ => None,
        }
    }

    /// The armour piece to draw: the vanity piece over the real one.
    pub(crate) fn shown_armor(&self, piece: usize) -> Option<ItemStack> {
        self.t5.tr.style.vanity[piece].or(self.surv.armor[piece])
    }

    // -----------------------------------------------------------------------------------------
    // Paint.

    /// The block looked at (within 8 blocks) and the face's normal.
    fn looked_block(&self) -> Option<(IVec3, IVec3)> {
        let eye = self.body.eye();
        let f = self.forward();
        let mut prev = eye.floor().as_ivec3();
        for k in 1..160 {
            let c = (eye + f * (k as f32 * 0.05)).floor().as_ivec3();
            if c == prev {
                continue;
            }
            if self.world.block_or_air(c) != voxel::AIR && !self.world.is_liquid(self.world.block_or_air(c)) {
                return Some((c, prev - c));
            }
            prev = c;
        }
        None
    }

    fn paint(&mut self, roller: bool, scrape: bool) -> bool {
        let Some((hit, n)) = self.looked_block() else { return true };
        // The dye: the first in the hotbar, then anywhere.
        let dye = if scrape {
            None
        } else {
            let slots: Vec<usize> = (0..9).chain(9..self.surv.inv.slots.len()).collect();
            let found = slots.into_iter().find_map(|i| {
                let st = self.surv.inv.slots[i]?;
                let name = &self.items.get(st.item).name;
                let c = name.strip_suffix("_dye")?;
                COLOURS.iter().position(|x| *x == c).map(|k| (i, k))
            });
            let Some(found) = found else {
                self.toast("Ponha uma tinta (corante) na barra para pintar", 2.0);
                return true;
            };
            Some(found)
        };
        let cells: Vec<IVec3> = if roller {
            let (a, b) = if n.x != 0 { (IVec3::Y, IVec3::Z) } else if n.y != 0 { (IVec3::X, IVec3::Z) } else { (IVec3::X, IVec3::Y) };
            (-1..=1).flat_map(|i| (-1..=1).map(move |j| hit + a * i + b * j)).collect()
        } else {
            vec![hit]
        };
        let mut done = 0;
        for c in cells {
            let name = self.reg.get(self.world.block_or_air(c)).name.clone();
            let Some(fam) = family(&name) else { continue };
            let (suffix, plain) = FAMILIES[fam];
            let to = match dye {
                Some((_, k)) => format!("{}{}", COLOURS[k], suffix),
                None => plain.to_string(),
            };
            if to == name {
                continue;
            }
            let id = self.reg.id_or_air(&to);
            if id == voxel::AIR {
                continue;
            }
            self.world.set_block(c, id);
            done += 1;
            let (layer, tint) = self.block_look(id);
            self.particles.block_bits(c, layer, tint, 3, 0.5);
        }
        if done == 0 {
            if !roller {
                self.toast("Isso não pega tinta (lã, tapete, concreto, terracota e vidro pegam)", 2.0);
            }
            return true;
        }
        self.sfx(Sfx::Place(Material::Cloth));
        if let (Some((slot, _)), true) = (dye, self.survival) {
            let s = &mut self.t5.tr.style;
            s.paint_uses += done;
            while s.paint_uses >= 8 {
                s.paint_uses -= 8;
                self.surv.inv.take(slot, 1);
            }
        }
        true
    }

    // -----------------------------------------------------------------------------------------
    // Music boxes.

    pub(crate) fn use_music_box(&mut self, p: IVec3) -> bool {
        let Some(song) = self.t5.tr.style.boxes.get(&p).cloned() else {
            let now = self.audio.as_ref().and_then(|a| a.now_playing.clone());
            match now {
                Some(t) => {
                    self.toast(format!("A caixa gravou: {t}"), 3.0);
                    self.t5.tr.style.boxes.insert(p, t);
                    self.sfx(Sfx::Click);
                }
                None => self.toast("Nenhuma música tocando agora para gravar", 2.0),
            }
            return true;
        };
        if self.t5.tr.style.playing == Some(p) {
            self.t5.tr.style.playing = None;
            if let Some(a) = self.audio.as_mut() {
                a.stop_disc();
            }
            return true;
        }
        self.extras.playing = None;
        if self.audio.as_mut().is_some_and(|a| a.play_title_on_disc(&song)) {
            self.t5.tr.style.playing = Some(p);
            self.toast(format!("Caixa de música: {song}"), 2.5);
        }
        true
    }

    pub(crate) fn update_tr_style(&mut self) {
        let Some(p) = self.t5.tr.style.playing else { return };
        let there = self.reg.get(self.world.block_or_air(p)).name == "music_box";
        if !there {
            self.t5.tr.style.boxes.remove(&p);
        }
        let d = (p.as_vec3() + Vec3::splat(0.5)).distance(self.body.eye());
        let Some(song) = self.t5.tr.style.boxes.get(&p).cloned() else {
            self.t5.tr.style.playing = None;
            if let Some(a) = self.audio.as_mut() {
                a.stop_disc();
            }
            return;
        };
        if self.extras.playing.is_some() {
            // A jukebox took over the record player.
            self.t5.tr.style.playing = None;
            return;
        }
        let vol = self.settings.music_volume.max(0.2);
        let Some(a) = self.audio.as_mut() else { return };
        if !a.disc_playing() {
            // On repeat.
            a.play_title_on_disc(&song);
        }
        let k = (1.0 - d / RANGE).clamp(0.0, 1.0);
        a.set_disc_volume(vol * k * k);
        if self.surv.rng.f32() < 0.02 {
            let glow = self.glow_layer();
            self.particles.glow(p.as_vec3() + Vec3::new(0.5, 1.2, 0.5), Vec3::Y * 0.6, glow, [1.0, 0.8, 0.5], 0.06, 1.5);
        }
    }

    /// Test scenes: MC_TR=vanity (the player in the pirate outfit, seen from the front) and
    /// paint (wool and glass painted in a row).
    pub(crate) fn test_tr_style(&mut self, scene: &str) {
        if scene == "vanity" {
            let set = std::env::var("MC_VANITY").unwrap_or_else(|_| "pirate".into());
            for (k, piece) in ["helmet", "chestplate", "leggings", "boots"].iter().enumerate() {
                self.t5.tr.style.vanity[k] = self.items.id(&format!("vanity_{set}_{piece}")).map(|id| ItemStack::new(id, 1));
            }
            return;
        }
        let f = self.forward();
        let flat = Vec3::new(f.x, 0.0, f.z).normalize_or_zero();
        let side = Vec3::new(-flat.z, 0.0, flat.x);
        let base = self.body.pos + flat * 5.0;
        for (k, c) in COLOURS.iter().enumerate() {
            let p = (base + side * (k as f32 - 8.0)).floor().as_ivec3();
            let fam = ["_wool", "_concrete", "_stained_glass", "_terracotta"][k % 4];
            self.world.set_block(p, self.reg.id_or_air(&format!("{c}{fam}")));
            self.world.set_block(p + IVec3::Y, self.reg.id_or_air(&format!("{c}_wool")));
        }
        let mb = self.reg.id_or_air("music_box");
        self.world.set_block((base + flat * 2.0).floor().as_ivec3(), mb);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn families_and_pieces() {
        assert_eq!(family("red_wool"), Some(0));
        assert_eq!(family("light_blue_concrete_powder"), Some(2));
        assert_eq!(family("light_blue_concrete"), Some(3));
        assert_eq!(family("glass"), Some(5));
        assert_eq!(family("lime_stained_glass_pane"), Some(4));
        assert_eq!(family("terracotta"), Some(6));
        assert_eq!(family("stone"), None);
        assert_eq!(piece_of("vanity_pirate_boots"), Some(3));
        assert_eq!(piece_of("iron_boots"), None);
    }
}
