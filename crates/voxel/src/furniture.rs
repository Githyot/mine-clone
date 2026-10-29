//! Furniture in the style of its wood (a Terraria idea): every wood builds its chairs, tables,
//! benches and shelves its own way, with its own legs, backs and ornaments, not just its own
//! colour. Oak is a farmhouse ladder-back, birch light Scandinavian, spruce a log cabin, jungle
//! tribal with a totem, dark oak a cushioned throne, cherry a Ming yoke-back, and so on.
//!
//! A block names its style in the data file (`style: Noble`) and may name an `accent` texture
//! (cushions, bark, sculk...) that the boxes after `Styled::accent` are drawn with. Bodies still
//! bump into the plain shapes, so sitting and walking feel the same whatever the wood.

use serde::Deserialize;

use crate::block::Shape;
use crate::shapes::{PBox, CONN_NX, CONN_NZ, CONN_PX, CONN_PZ};

#[derive(Deserialize, Clone, Copy, Debug, PartialEq, Eq, Default, Hash)]
pub enum Style {
    /// The simple shapes every wood shared before.
    #[default]
    Plain,
    /// Oak: farmhouse ladder-back, stretchers between the legs, thick trestle benches.
    Rustic,
    /// Birch: light Scandinavian, thin tapered legs, spindle backs, slatted seats.
    Slim,
    /// Spruce: log cabin, legs and rails of bark-covered logs.
    Cabin,
    /// Jungle: tribal, drum pedestals, a totem face on the chair back, teeth along the edges.
    Tribal,
    /// Acacia: savanna carving, splayed stepped legs, a diamond fret in the back.
    Carved,
    /// Dark oak: noble, turned legs with beads, high crested back, red cushions, a tablecloth.
    Noble,
    /// Cherry: Ming style, yoke back with up-turned ends, hoof feet, spandrels.
    Oriental,
    /// Mangrove: roots splaying from the legs, woven back.
    Root,
    /// Pale oak: gothic, pointed arches, spires, black velvet.
    Gothic,
    /// Crimson: mushroom furniture, caps on stems.
    Fungal,
    /// Warped: twisting trunks, lopsided tendrils, wart knobs.
    Twisted,
    /// Maple: Shaker rocking chair, tapered legs, knob finials, an autumn cushion.
    Shaker,
    /// Walnut: mid-century modern, splayed legs, floating shell seat, teal cushion.
    Modern,
    /// Araucaria: Brazilian colonial, turned spindles, arched crest, pine-cone finials.
    Colonial,
    /// Buriti: woven palm-fibre wicker, a barrel back and skirt.
    Wicker,
    /// Coconut: beach, Adirondack fan back and wide arms, thatch fringes.
    Beach,
    /// Ipê: hard deck wood, sled sides, slatted seats, panel legs.
    Deck,
    /// Jabuticaba: rococo, cabriole legs, shield back, purple cushions, a fruit cluster.
    Rococo,
    /// Echo: ancient city, stone-like pillars with sculk veins.
    Ancient,
    /// Skyroot: celestial, thin legs on cloud puffs, winged backs.
    Celestial,
}

impl Style {
    pub const ALL: [Style; 21] = [
        Style::Plain,
        Style::Rustic,
        Style::Slim,
        Style::Cabin,
        Style::Tribal,
        Style::Carved,
        Style::Noble,
        Style::Oriental,
        Style::Root,
        Style::Gothic,
        Style::Fungal,
        Style::Twisted,
        Style::Shaker,
        Style::Modern,
        Style::Colonial,
        Style::Wicker,
        Style::Beach,
        Style::Deck,
        Style::Rococo,
        Style::Ancient,
        Style::Celestial,
    ];
}

/// Shapes that change with the style.
pub fn styled(shape: Shape) -> bool {
    matches!(
        shape,
        Shape::Chair | Shape::Table | Shape::Bench | Shape::Shelf
    )
}

/// Boxes under construction: the wood (`m`) and the accent (`a`).
#[derive(Default)]
struct D {
    m: Vec<PBox>,
    a: Vec<PBox>,
}

fn mx(b: PBox) -> PBox {
    [16 - b[3], b[1], b[2], 16 - b[0], b[4], b[5]]
}

fn mz(b: PBox) -> PBox {
    [b[0], b[1], 16 - b[5], b[3], b[4], 16 - b[2]]
}

/// Turn a box drawn facing north by `k` quarter turns (north -> east -> south -> west).
fn rot(b: PBox, k: u8) -> PBox {
    let mut b = b;
    for _ in 0..k {
        let [x0, y0, z0, x1, y1, z1] = b;
        b = [16 - z1, y0, x0, 16 - z0, y1, x1];
    }
    b
}

impl D {
    fn m(&mut self, bs: &[PBox]) -> &mut Self {
        self.m.extend_from_slice(bs);
        self
    }
    fn a(&mut self, bs: &[PBox]) -> &mut Self {
        self.a.extend_from_slice(bs);
        self
    }
    /// The boxes and their mirror across x (left and right).
    fn mx(&mut self, bs: &[PBox]) -> &mut Self {
        for &b in bs {
            self.m.push(b);
            self.m.push(mx(b));
        }
        self
    }
    fn ax(&mut self, bs: &[PBox]) -> &mut Self {
        for &b in bs {
            self.a.push(b);
            self.a.push(mx(b));
        }
        self
    }
    /// The boxes at all four corners (mirrored across x and z).
    fn m4(&mut self, bs: &[PBox]) -> &mut Self {
        for &b in bs {
            self.m.extend([b, mx(b), mz(b), mx(mz(b))]);
        }
        self
    }
    fn a4(&mut self, bs: &[PBox]) -> &mut Self {
        for &b in bs {
            self.a.extend([b, mx(b), mz(b), mx(mz(b))]);
        }
        self
    }
}

/// Boxes of a styled shape that is drawn (not collided with): the wood boxes first, then the
/// accent ones, starting at the returned index.
pub fn boxes(shape: Shape, style: Style, variant: u8, conn: u8, out: &mut Vec<PBox>) -> usize {
    let f = crate::shapes::facing_of(variant);
    let d = match shape {
        Shape::Chair => turned(chair(style), f),
        Shape::Bench => turned(bench(style), f),
        Shape::Shelf => turned(shelf(style), f),
        Shape::Table => table(style, conn),
        _ => D::default(),
    };
    out.extend(d.m);
    let accent = out.len();
    out.extend(d.a);
    accent
}

fn turned(d: D, f: u8) -> D {
    D {
        m: d.m.into_iter().map(|b| rot(b, f)).collect(),
        a: d.a.into_iter().map(|b| rot(b, f)).collect(),
    }
}

/// A chair drawn facing north: the sitter looks to -z, the back is on the south side. The seat
/// top stays at 10 pixels so everyone sits at the same height.
fn chair(s: Style) -> D {
    let mut d = D::default();
    match s {
        Style::Plain => {
            d.m4(&[[2, 0, 2, 4, 8, 4]]).m(&[
                [2, 8, 2, 14, 10, 14],
                [2, 10, 12, 4, 16, 14],
                [12, 10, 12, 14, 16, 14],
                [4, 12, 12, 12, 15, 13],
            ]);
        }
        Style::Rustic => {
            // Farmhouse ladder-back: back posts from the floor, stretchers, three slats.
            d.mx(&[
                [2, 0, 2, 4, 8, 4],
                [2, 0, 12, 4, 16, 14],
                [2, 3, 4, 4, 4, 12],
            ])
            .m(&[
                [2, 8, 2, 14, 10, 14],
                [4, 2, 2, 12, 3, 3],
                [4, 11, 12, 12, 12, 13],
                [4, 13, 12, 12, 14, 13],
                [1, 15, 12, 15, 16, 14],
            ]);
        }
        Style::Slim => {
            // Tapered legs (thin at the foot), a thin seat, spindle back with a top rail.
            d.m4(&[[3, 0, 3, 4, 5, 4], [2, 5, 2, 4, 9, 4]])
                .m(&[[2, 9, 2, 14, 10, 14]])
                .mx(&[[2, 10, 13, 3, 16, 14], [5, 10, 13, 6, 15, 14]])
                .m(&[[7, 10, 13, 9, 15, 14], [2, 15, 12, 14, 16, 14]]);
        }
        Style::Cabin => {
            // Thick log legs and rails with their bark on, a heavy plank seat.
            d.a4(&[[1, 0, 1, 4, 7, 4]])
                .ax(&[[1, 7, 12, 4, 16, 15], [2, 3, 4, 3, 5, 12]])
                .a(&[[4, 11, 13, 12, 13, 15], [4, 14, 13, 12, 16, 15]])
                .m(&[[1, 7, 1, 15, 10, 12], [4, 7, 12, 12, 10, 14]]);
        }
        Style::Tribal => {
            // Wide feet, a toothed seat edge and a totem with eyes, nose and horns.
            d.a4(&[[1, 0, 1, 5, 2, 5]])
                .m4(&[[2, 2, 2, 4, 8, 4]])
                .m(&[
                    [1, 8, 1, 15, 10, 14],
                    [3, 7, 1, 5, 8, 2],
                    [7, 7, 1, 9, 8, 2],
                    [11, 7, 1, 13, 8, 2],
                ])
                .m(&[[5, 10, 12, 11, 16, 14], [7, 11, 11, 9, 13, 12]])
                .mx(&[
                    [2, 10, 12, 4, 13, 14],
                    [6, 13, 11, 7, 14, 12],
                    [4, 14, 12, 5, 16, 13],
                ]);
        }
        Style::Carved => {
            // Splayed stepped legs, a notched apron, a diamond fret under a crested rail.
            d.m4(&[[1, 0, 1, 3, 3, 3], [2, 3, 2, 4, 6, 4], [3, 6, 3, 5, 8, 5]])
                .m(&[[2, 8, 2, 14, 10, 14], [2, 7, 2, 14, 8, 3]])
                .mx(&[
                    [4, 6, 2, 6, 7, 3],
                    [2, 10, 12, 4, 16, 14],
                    [6, 11, 12, 7, 12, 13],
                    [5, 12, 12, 6, 13, 13],
                    [6, 13, 12, 7, 14, 13],
                ])
                .m(&[
                    [7, 10, 12, 9, 11, 13],
                    [7, 14, 12, 9, 15, 13],
                    [2, 15, 12, 14, 16, 14],
                    [6, 14, 12, 10, 15, 14],
                ]);
        }
        Style::Noble => {
            // Plinth feet, beaded legs, arms, a tall crested back, red cushions.
            d.m4(&[[1, 0, 1, 5, 1, 5], [2, 1, 2, 4, 8, 4], [1, 4, 1, 5, 5, 5]])
                .m(&[
                    [1, 8, 1, 15, 9, 14],
                    [3, 9, 14, 13, 15, 15],
                    [3, 15, 13, 13, 16, 15],
                ])
                .mx(&[
                    [1, 8, 13, 3, 16, 15],
                    [1, 12, 3, 3, 13, 13],
                    [1, 9, 3, 3, 12, 5],
                ])
                .a(&[[3, 9, 2, 13, 10, 13], [4, 10, 13, 12, 14, 14]]);
        }
        Style::Oriental => {
            // Hoof feet, stepped stretchers, spandrels, a yoke rail with up-turned ends.
            d.m4(&[[1, 0, 1, 4, 1, 4], [2, 1, 2, 4, 8, 4]])
                .m(&[
                    [4, 1, 2, 12, 2, 3],
                    [4, 3, 13, 12, 4, 14],
                    [2, 8, 2, 14, 10, 14],
                    [4, 7, 2, 12, 8, 3],
                    [7, 10, 12, 9, 14, 13],
                    [6, 12, 12, 10, 13, 13],
                ])
                .mx(&[
                    [2, 2, 4, 3, 3, 12],
                    [4, 6, 2, 5, 7, 3],
                    [2, 10, 12, 4, 14, 14],
                    [0, 14, 12, 8, 15, 14],
                    [0, 15, 12, 1, 16, 14],
                ]);
        }
        Style::Root => {
            // Legs that split into roots at the floor; a woven back.
            d.a4(&[[1, 0, 1, 4, 2, 4], [0, 0, 2, 1, 1, 3], [2, 0, 0, 3, 1, 1]])
                .m4(&[[2, 2, 2, 4, 8, 4]])
                .m(&[[2, 8, 2, 14, 10, 14], [4, 6, 2, 12, 7, 3]])
                .mx(&[[2, 10, 12, 4, 16, 14], [6, 10, 13, 7, 16, 14]])
                .m(&[
                    [4, 11, 12, 12, 12, 13],
                    [4, 13, 12, 12, 14, 13],
                    [4, 15, 12, 12, 16, 13],
                ]);
        }
        Style::Gothic => {
            // Straight legs, a pointed arch in the back over black velvet, spired posts.
            d.m4(&[[2, 0, 2, 4, 8, 4]])
                .m(&[
                    [2, 8, 2, 14, 9, 14],
                    [4, 10, 12, 12, 11, 13],
                    [7, 15, 12, 9, 16, 13],
                ])
                .mx(&[
                    [2, 9, 12, 4, 14, 14],
                    [1, 14, 11, 5, 15, 15],
                    [2, 15, 12, 4, 16, 14],
                    [4, 11, 12, 5, 13, 13],
                    [5, 13, 12, 6, 14, 13],
                    [6, 14, 12, 7, 15, 13],
                ])
                .a(&[
                    [3, 9, 3, 13, 10, 13],
                    [5, 11, 13, 11, 14, 14],
                    [6, 14, 13, 10, 15, 14],
                    [7, 15, 13, 9, 16, 14],
                ]);
        }
        Style::Fungal => {
            // A cap on a stem to sit on, and a smaller cap for a back.
            d.m(&[
                [5, 0, 5, 11, 1, 11],
                [6, 1, 6, 10, 8, 10],
                [3, 7, 3, 13, 8, 13],
                [6, 10, 13, 10, 13, 15],
            ])
            .a(&[
                [1, 8, 1, 15, 10, 15],
                [3, 13, 12, 13, 15, 16],
                [5, 15, 13, 11, 16, 15],
            ]);
        }
        Style::Twisted => {
            // Legs that twist as they rise; a lopsided back with wart knobs.
            d.m4(&[[2, 0, 3, 4, 3, 5], [3, 3, 2, 5, 6, 4], [2, 6, 2, 4, 8, 4]])
                .m(&[
                    [2, 8, 2, 14, 10, 14],
                    [2, 10, 12, 4, 13, 14],
                    [3, 13, 12, 5, 15, 14],
                    [4, 15, 12, 8, 16, 14],
                    [12, 10, 12, 14, 14, 14],
                    [11, 14, 12, 13, 16, 14],
                ])
                .a(&[[8, 13, 11, 11, 16, 14], [1, 9, 1, 3, 11, 3]]);
        }
        Style::Shaker => {
            // A rocking chair: curved runners, arms, a tilted ladder back, knob finials.
            d.mx(&[
                [2, 1, 0, 4, 2, 2],
                [2, 0, 2, 4, 1, 12],
                [2, 1, 12, 4, 2, 15],
                [2, 2, 15, 4, 3, 16],
                [2, 1, 3, 4, 12, 5],
                [2, 1, 11, 4, 8, 13],
                [1, 12, 3, 4, 13, 12],
                [2, 8, 12, 4, 13, 14],
                [2, 13, 13, 4, 16, 15],
            ])
            .m(&[
                [2, 8, 3, 14, 9, 14],
                [4, 10, 12, 12, 11, 14],
                [4, 12, 13, 12, 13, 15],
                [4, 14, 13, 12, 15, 15],
            ])
            .a(&[[3, 9, 4, 13, 10, 13]]);
        }
        Style::Modern => {
            // Splayed slim legs, a floating shell held by a spine, teal cushions.
            d.m4(&[[1, 0, 1, 3, 3, 3], [2, 3, 2, 4, 6, 4], [3, 6, 3, 5, 8, 5]])
                .m(&[
                    [2, 8, 2, 14, 9, 14],
                    [2, 9, 13, 14, 15, 14],
                    [7, 6, 9, 9, 8, 14],
                ])
                .mx(&[[2, 10, 11, 3, 14, 13], [3, 15, 13, 5, 16, 14]])
                .a(&[[3, 9, 2, 13, 10, 13], [3, 10, 12, 13, 14, 13]]);
        }
        Style::Colonial => {
            // Legs with beads and an H stretcher, turned spindles, an arched crest, pine cones.
            d.m4(&[[2, 0, 2, 4, 8, 4], [1, 6, 1, 5, 7, 5]])
                .mx(&[
                    [2, 2, 4, 4, 3, 12],
                    [2, 10, 12, 4, 14, 14],
                    [5, 10, 12, 6, 14, 13],
                ])
                .m(&[
                    [4, 2, 7, 12, 3, 9],
                    [1, 8, 1, 15, 10, 14],
                    [7, 10, 12, 9, 14, 13],
                    [4, 14, 12, 12, 15, 14],
                    [6, 15, 12, 10, 16, 14],
                ])
                .ax(&[[1, 14, 11, 4, 15, 15], [2, 15, 12, 3, 16, 14]]);
        }
        Style::Wicker => {
            // A woven barrel: skirt, round back and arms, wooden rims.
            d.a(&[
                [2, 1, 2, 14, 7, 14],
                [3, 8, 2, 13, 10, 12],
                [1, 8, 12, 15, 15, 15],
            ])
            .ax(&[[1, 8, 2, 3, 12, 12]])
            .m(&[
                [1, 0, 1, 15, 1, 15],
                [1, 7, 1, 15, 8, 15],
                [1, 15, 12, 15, 16, 15],
            ])
            .mx(&[[1, 12, 2, 3, 13, 12]]);
        }
        Style::Beach => {
            // Adirondack: wide flat arms, a low seat, a fan of slats with lashed front legs.
            d.mx(&[
                [2, 0, 2, 4, 11, 4],
                [2, 0, 10, 4, 7, 12],
                [0, 11, 2, 4, 12, 13],
                [3, 9, 12, 5, 13, 13],
                [3, 13, 13, 5, 15, 14],
            ])
            .m(&[
                [2, 9, 2, 14, 10, 7],
                [2, 8, 7, 14, 9, 13],
                [7, 9, 12, 9, 13, 13],
                [7, 13, 13, 9, 16, 14],
                [2, 10, 12, 14, 11, 14],
            ])
            .ax(&[[1, 5, 1, 5, 6, 5]]);
        }
        Style::Deck => {
            // Sled sides with runners, a slatted deck seat and back.
            d.mx(&[
                [1, 0, 1, 3, 1, 15],
                [1, 1, 2, 3, 10, 4],
                [1, 1, 12, 3, 16, 14],
                [1, 9, 4, 3, 10, 12],
            ])
            .m(&[
                [3, 8, 2, 13, 10, 5],
                [3, 8, 6, 13, 10, 9],
                [3, 8, 10, 13, 10, 13],
                [3, 11, 13, 13, 13, 14],
                [3, 14, 13, 13, 16, 14],
            ]);
        }
        Style::Rococo => {
            // Cabriole legs (knee out, ankle in, foot out), a shield back, purple cushions.
            d.m4(&[[1, 0, 1, 3, 1, 3], [2, 1, 2, 4, 5, 4], [1, 5, 1, 3, 8, 3]])
                .m(&[
                    [1, 8, 1, 15, 9, 14],
                    [3, 10, 12, 13, 11, 13],
                    [4, 15, 12, 12, 16, 13],
                ])
                .mx(&[[2, 9, 12, 3, 14, 13], [3, 14, 12, 4, 15, 13]])
                .a(&[
                    [2, 9, 2, 14, 10, 13],
                    [3, 11, 11, 13, 14, 13],
                    [4, 14, 11, 12, 15, 13],
                ]);
        }
        Style::Ancient => {
            // Pillar legs with caps and bases, a gate-like back framing a sculk panel.
            d.m4(&[[1, 0, 1, 5, 1, 5], [2, 1, 2, 4, 7, 4], [1, 7, 1, 5, 8, 5]])
                .m(&[
                    [2, 8, 2, 14, 10, 14],
                    [1, 15, 12, 15, 16, 15],
                    [7, 12, 12, 9, 14, 13],
                ])
                .mx(&[[2, 10, 12, 4, 15, 14]])
                .a(&[[4, 10, 13, 12, 15, 14], [5, 9, 3, 11, 10, 4]]);
        }
        Style::Celestial => {
            // Thin legs standing on cloud puffs, a winged back on a central post.
            d.a4(&[[1, 0, 1, 5, 2, 5], [2, 2, 2, 4, 3, 4]])
                .m4(&[[2, 3, 2, 3, 8, 3]])
                .m(&[[2, 8, 2, 14, 10, 14], [7, 10, 12, 9, 16, 14]])
                .mx(&[
                    [3, 11, 12, 7, 13, 13],
                    [1, 12, 12, 3, 15, 13],
                    [4, 13, 12, 7, 15, 13],
                    [2, 15, 12, 5, 16, 13],
                ]);
        }
    }
    d
}

/// A bench drawn running east-west, its back (when it has one) on the south side. The seat top
/// is at 9 pixels.
fn bench(s: Style) -> D {
    let mut d = D::default();
    let seat = [0, 7, 3, 16, 9, 13];
    match s {
        Style::Plain => {
            d.m(&[seat]).mx(&[[1, 0, 4, 3, 7, 12]]);
        }
        Style::Rustic => {
            // Trestle ends cut into an arch, a stretcher between them.
            d.m(&[seat, [3, 3, 7, 13, 4, 9]]).mx(&[
                [1, 0, 4, 3, 7, 6],
                [1, 0, 10, 3, 7, 12],
                [1, 5, 6, 3, 7, 10],
            ]);
        }
        Style::Slim => {
            d.m(&[
                [0, 8, 3, 16, 9, 6],
                [0, 8, 7, 16, 9, 9],
                [0, 8, 10, 16, 9, 13],
            ])
            .mx(&[[2, 7, 3, 3, 8, 13]])
            .m4(&[[3, 0, 4, 4, 4, 5], [2, 4, 3, 4, 7, 5]]);
        }
        Style::Cabin => {
            // Half a log on two stumps, planed flat on top.
            d.m(&[[0, 8, 4, 16, 9, 12]])
                .a(&[[0, 6, 4, 16, 8, 12]])
                .ax(&[[2, 0, 5, 5, 6, 11]]);
        }
        Style::Tribal => {
            d.m(&[seat])
                .ax(&[[1, 0, 4, 5, 7, 12]])
                .mx(&[[0, 2, 3, 6, 3, 13], [0, 5, 3, 6, 6, 13]])
                .m(&[[3, 6, 3, 5, 7, 4], [7, 6, 3, 9, 7, 4], [11, 6, 3, 13, 7, 4]]);
        }
        Style::Carved => {
            // Splayed stepped legs and curled arm ends.
            d.m(&[seat])
                .m4(&[[0, 0, 3, 2, 3, 5], [1, 3, 4, 3, 7, 6]])
                .mx(&[
                    [0, 9, 3, 2, 10, 13],
                    [0, 10, 3, 1, 11, 13],
                    [4, 6, 3, 6, 7, 4],
                ]);
        }
        Style::Noble => {
            // A settee: ball feet, padded seat and back, arms.
            d.m(&[[0, 6, 3, 16, 8, 13], [0, 8, 12, 16, 15, 14]])
                .m4(&[[0, 0, 3, 2, 6, 5]])
                .mx(&[[0, 8, 3, 1, 11, 12]])
                .a(&[[1, 8, 3, 15, 9, 12], [1, 9, 11, 15, 14, 12]]);
        }
        Style::Oriental => {
            d.m(&[seat, [3, 5, 3, 13, 7, 4], [3, 5, 12, 13, 7, 13]])
                .m4(&[[1, 1, 3, 3, 7, 5], [2, 0, 4, 4, 1, 6]]);
        }
        Style::Root => {
            d.m(&[seat])
                .a4(&[[1, 0, 3, 4, 2, 6], [0, 0, 2, 2, 1, 3]])
                .mx(&[[1, 2, 4, 3, 7, 12]]);
        }
        Style::Gothic => {
            // A pew: pointed-arch ends, a back with pinnacles.
            d.m(&[seat, [0, 9, 12, 16, 14, 13]])
                .mx(&[
                    [1, 0, 3, 3, 7, 5],
                    [1, 0, 11, 3, 7, 13],
                    [1, 5, 5, 3, 6, 6],
                    [1, 6, 6, 3, 7, 10],
                    [0, 9, 12, 2, 16, 14],
                ])
                .a(&[[2, 9, 13, 14, 13, 14]]);
        }
        Style::Fungal => {
            d.a(&[[0, 7, 2, 16, 9, 14]])
                .m(&[[1, 6, 4, 15, 7, 12]])
                .mx(&[[2, 0, 6, 6, 6, 10]]);
        }
        Style::Twisted => {
            d.m(&[seat])
                .m4(&[[1, 0, 4, 3, 3, 6], [2, 3, 3, 4, 7, 5]])
                .a(&[[13, 9, 4, 16, 10, 7], [0, 9, 9, 2, 11, 12]]);
        }
        Style::Shaker => {
            // Spindle back with a top rail, an autumn cushion.
            d.m(&[[0, 7, 3, 16, 8, 13], [0, 14, 12, 16, 15, 14]])
                .m4(&[[1, 0, 4, 3, 7, 6]])
                .mx(&[
                    [1, 8, 12, 2, 14, 13],
                    [4, 8, 12, 5, 14, 13],
                    [7, 8, 12, 8, 14, 13],
                ])
                .a(&[[1, 8, 4, 15, 9, 12]]);
        }
        Style::Modern => {
            d.m(&[[0, 7, 3, 16, 8, 13]])
                .m4(&[[1, 0, 3, 2, 3, 4], [2, 3, 4, 3, 7, 5]])
                .a(&[[1, 8, 3, 15, 9, 13]]);
        }
        Style::Colonial => {
            d.m(&[seat, [3, 2, 7, 13, 3, 9], [0, 13, 12, 16, 14, 14]])
                .m4(&[[1, 0, 4, 3, 7, 6], [0, 5, 3, 4, 6, 7]])
                .mx(&[[1, 9, 12, 3, 13, 14], [5, 9, 12, 6, 13, 13]])
                .ax(&[[1, 14, 12, 3, 16, 14]]);
        }
        Style::Wicker => {
            d.a(&[
                [1, 0, 3, 15, 7, 13],
                [1, 8, 3, 15, 9, 12],
                [0, 8, 12, 16, 14, 14],
            ])
            .m(&[[0, 7, 3, 16, 8, 13], [0, 14, 12, 16, 15, 14]])
            .ax(&[[0, 8, 3, 1, 12, 12]]);
        }
        Style::Beach => {
            // Driftwood plank on posts lashed with thatch.
            d.m(&[[0, 7, 3, 16, 9, 13]])
                .mx(&[[2, 0, 5, 5, 7, 11]])
                .ax(&[[1, 3, 4, 6, 4, 12], [1, 5, 4, 6, 6, 12]]);
        }
        Style::Deck => {
            d.m(&[
                [2, 7, 3, 14, 9, 6],
                [2, 7, 7, 14, 9, 9],
                [2, 7, 10, 14, 9, 13],
            ])
            .mx(&[[0, 0, 3, 2, 9, 13]]);
        }
        Style::Rococo => {
            d.m(&[[0, 7, 3, 16, 8, 13], [0, 6, 3, 16, 7, 4]])
                .m4(&[[0, 0, 3, 2, 1, 5], [1, 1, 4, 3, 4, 6], [0, 4, 3, 2, 7, 5]])
                .a(&[[0, 8, 3, 16, 9, 13], [7, 5, 2, 9, 7, 3]]);
        }
        Style::Ancient => {
            d.m(&[seat])
                .mx(&[[1, 0, 4, 5, 7, 12], [0, 0, 3, 6, 1, 13]])
                .ax(&[[2, 2, 3, 4, 6, 4]])
                .a(&[[3, 9, 7, 13, 10, 9]]);
        }
        Style::Celestial => {
            d.m(&[seat]).mx(&[[2, 2, 7, 4, 7, 9]]).ax(&[
                [0, 0, 5, 6, 2, 11],
                [1, 6, 2, 5, 8, 4],
                [1, 6, 12, 5, 8, 14],
            ]);
        }
    }
    d
}

/// A shelf drawn against a wall to the north: a board and what holds it up.
fn shelf(s: Style) -> D {
    let mut d = D::default();
    match s {
        Style::Plain => {
            d.m(&[[0, 8, 0, 16, 10, 7]]).mx(&[[2, 4, 0, 4, 8, 2]]);
        }
        Style::Rustic => {
            d.m(&[[0, 8, 0, 16, 10, 7]]).mx(&[
                [2, 3, 0, 4, 8, 2],
                [2, 6, 2, 4, 8, 4],
                [2, 7, 4, 4, 8, 6],
            ]);
        }
        Style::Slim => {
            d.m(&[[0, 9, 0, 16, 10, 6]])
                .mx(&[[3, 5, 0, 4, 9, 1], [3, 8, 1, 4, 9, 5]]);
        }
        Style::Cabin => {
            d.m(&[[0, 7, 0, 16, 10, 7]])
                .ax(&[[2, 2, 0, 5, 7, 3], [2, 5, 3, 5, 7, 6]]);
        }
        Style::Tribal => {
            d.m(&[
                [0, 8, 0, 16, 10, 7],
                [1, 7, 6, 3, 8, 7],
                [5, 7, 6, 7, 8, 7],
                [9, 7, 6, 11, 8, 7],
                [13, 7, 6, 15, 8, 7],
            ])
            .ax(&[[2, 3, 0, 4, 7, 3]]);
        }
        Style::Carved => {
            d.m(&[[0, 8, 0, 16, 10, 7], [0, 10, 0, 16, 11, 1]]).mx(&[
                [2, 4, 0, 4, 8, 2],
                [2, 6, 2, 4, 8, 4],
                [2, 5, 0, 4, 6, 3],
            ]);
        }
        Style::Noble => {
            // Crown moulding and scrolled corbels.
            d.m(&[
                [0, 8, 0, 16, 10, 7],
                [0, 10, 5, 16, 11, 8],
                [0, 7, 6, 16, 8, 8],
            ])
            .mx(&[[2, 2, 0, 4, 8, 2], [2, 6, 2, 4, 8, 5], [2, 2, 2, 4, 3, 3]])
            .a(&[[1, 10, 1, 15, 11, 5]]);
        }
        Style::Oriental => {
            d.m(&[
                [0, 9, 0, 16, 10, 7],
                [0, 10, 0, 1, 11, 7],
                [15, 10, 0, 16, 11, 7],
            ])
            .mx(&[[3, 4, 0, 4, 9, 1], [3, 6, 1, 4, 7, 5]]);
        }
        Style::Root => {
            d.m(&[[0, 8, 0, 16, 10, 7]]).ax(&[
                [2, 2, 0, 4, 8, 2],
                [3, 5, 2, 5, 8, 4],
                [1, 1, 0, 3, 2, 1],
            ]);
        }
        Style::Gothic => {
            d.m(&[[0, 8, 0, 16, 10, 7], [7, 5, 0, 9, 8, 1]]).mx(&[
                [2, 3, 0, 4, 8, 2],
                [4, 6, 0, 5, 8, 1],
                [5, 7, 0, 6, 8, 1],
                [2, 10, 0, 3, 13, 1],
            ]);
        }
        Style::Fungal => {
            d.a(&[[0, 8, 0, 16, 10, 7]])
                .m(&[[2, 7, 0, 14, 8, 5]])
                .mx(&[[3, 3, 0, 5, 7, 2]]);
        }
        Style::Twisted => {
            d.m(&[
                [0, 8, 0, 16, 10, 7],
                [2, 3, 0, 4, 6, 2],
                [3, 6, 1, 5, 8, 3],
                [12, 5, 0, 14, 8, 2],
            ])
            .a(&[[11, 2, 0, 14, 5, 1]]);
        }
        Style::Shaker => {
            // A peg rail under the board, like a Shaker wall.
            d.m(&[[0, 9, 0, 16, 10, 7], [0, 5, 0, 16, 7, 1]]).mx(&[
                [3, 5, 1, 4, 6, 3],
                [7, 5, 1, 8, 6, 3],
                [2, 7, 0, 3, 9, 3],
            ]);
        }
        Style::Modern => {
            d.m(&[[0, 9, 0, 16, 10, 7]])
                .a(&[[1, 10, 0, 6, 11, 5]])
                .mx(&[[1, 8, 0, 2, 9, 6]]);
        }
        Style::Colonial => {
            d.m(&[[0, 8, 0, 16, 10, 7], [0, 10, 0, 16, 11, 1]])
                .mx(&[[2, 3, 0, 4, 8, 2], [2, 6, 2, 4, 8, 4]])
                .ax(&[[2, 2, 0, 4, 3, 2]]);
        }
        Style::Wicker => {
            d.a(&[[0, 8, 0, 16, 10, 7]])
                .m(&[[0, 10, 6, 16, 11, 7]])
                .mx(&[[2, 4, 0, 4, 8, 2]]);
        }
        Style::Beach => {
            d.m(&[[0, 8, 0, 16, 10, 7]])
                .a(&[[0, 6, 6, 16, 8, 7]])
                .mx(&[[3, 5, 0, 5, 8, 2]]);
        }
        Style::Deck => {
            d.m(&[[0, 8, 0, 16, 10, 3], [0, 8, 4, 16, 10, 7]])
                .mx(&[[0, 2, 0, 2, 8, 6]]);
        }
        Style::Rococo => {
            d.m(&[[0, 8, 0, 16, 10, 7], [6, 6, 0, 10, 8, 2]])
                .mx(&[[2, 4, 0, 3, 8, 2], [2, 7, 2, 3, 8, 5], [1, 3, 0, 3, 4, 2]])
                .a(&[[7, 5, 0, 9, 6, 2]]);
        }
        Style::Ancient => {
            d.m(&[[0, 8, 0, 16, 10, 7], [0, 7, 0, 16, 8, 1]])
                .mx(&[[1, 2, 0, 4, 7, 3]])
                .a(&[[2, 10, 2, 14, 11, 3]]);
        }
        Style::Celestial => {
            d.m(&[[0, 8, 0, 16, 10, 7]])
                .ax(&[[1, 6, 0, 5, 8, 4], [2, 5, 0, 4, 6, 2]]);
        }
    }
    d
}

/// A table: tables side by side share their top, and only the open corners get legs and only
/// the open edges their aprons. Pieces are drawn for the north-west corner and the north edge.
struct TableParts {
    top: D,
    corner: D,
    edge: D,
    center: D,
}

fn table(s: Style, conn: u8) -> D {
    let p = table_parts(s);
    let mut d = p.top;
    d.m.extend(p.center.m);
    d.a.extend(p.center.a);
    // Corners: NW, NE, SW, SE.
    for (bits, fx, fz) in [
        (CONN_NX | CONN_NZ, false, false),
        (CONN_PX | CONN_NZ, true, false),
        (CONN_NX | CONN_PZ, false, true),
        (CONN_PX | CONN_PZ, true, true),
    ] {
        if conn & bits != 0 {
            continue;
        }
        let place = |b: PBox| {
            let b = if fx { mx(b) } else { b };
            if fz {
                mz(b)
            } else {
                b
            }
        };
        d.m.extend(p.corner.m.iter().map(|&b| place(b)));
        d.a.extend(p.corner.a.iter().map(|&b| place(b)));
    }
    // Edges: north, east, south, west.
    for (k, bit) in [CONN_NZ, CONN_PX, CONN_PZ, CONN_NX].into_iter().enumerate() {
        if conn & bit != 0 {
            continue;
        }
        d.m.extend(p.edge.m.iter().map(|&b| rot(b, k as u8)));
        d.a.extend(p.edge.a.iter().map(|&b| rot(b, k as u8)));
    }
    d
}

fn table_parts(s: Style) -> TableParts {
    let (mut top, mut corner, mut edge, mut center) =
        (D::default(), D::default(), D::default(), D::default());
    match s {
        Style::Plain => {
            top.m(&[[0, 13, 0, 16, 16, 16]]);
            corner.m(&[[1, 0, 1, 3, 13, 3]]);
        }
        Style::Rustic => {
            // Thick top, apron, chunky legs, a low stretcher round the frame.
            top.m(&[[0, 13, 0, 16, 16, 16]]);
            corner.m(&[[1, 0, 1, 4, 13, 4]]);
            edge.m(&[[0, 11, 1, 16, 13, 2], [0, 3, 2, 16, 4, 3]]);
        }
        Style::Slim => {
            top.m(&[[0, 15, 0, 16, 16, 16]]);
            corner.m(&[[2, 6, 2, 4, 15, 4], [3, 0, 3, 4, 6, 4]]);
            edge.m(&[[0, 14, 1, 16, 15, 2]]);
        }
        Style::Cabin => {
            top.m(&[[0, 13, 0, 16, 16, 16]]);
            corner.a(&[[1, 0, 1, 4, 13, 4]]);
            edge.a(&[[0, 11, 1, 16, 13, 3]]);
        }
        Style::Tribal => {
            // A drum pedestal banded with rings, teeth under the edge.
            top.m(&[[0, 13, 0, 16, 16, 16]]);
            center
                .m(&[
                    [3, 0, 3, 13, 2, 13],
                    [3, 4, 3, 13, 5, 13],
                    [3, 8, 3, 13, 9, 13],
                    [4, 10, 4, 12, 13, 12],
                ])
                .a(&[[4, 2, 4, 12, 10, 12]]);
            edge.m(&[
                [2, 12, 0, 4, 13, 1],
                [7, 12, 0, 9, 13, 1],
                [12, 12, 0, 14, 13, 1],
            ]);
        }
        Style::Carved => {
            top.m(&[[0, 14, 0, 16, 16, 16]]);
            corner.m(&[[1, 0, 1, 4, 2, 4], [2, 2, 2, 4, 12, 4]]);
            edge.m(&[
                [0, 12, 1, 16, 14, 2],
                [3, 11, 1, 5, 12, 2],
                [7, 11, 1, 9, 12, 2],
                [11, 11, 1, 13, 12, 2],
                [0, 13, 0, 16, 14, 1],
            ]);
        }
        Style::Noble => {
            // A red tablecloth draped over the edges, heavy beaded legs.
            top.m(&[[0, 14, 0, 16, 15, 16]])
                .a(&[[0, 15, 0, 16, 16, 16]]);
            corner.m(&[[1, 0, 1, 4, 1, 4], [2, 1, 2, 4, 12, 4], [1, 4, 1, 5, 7, 5]]);
            edge.a(&[
                [0, 12, 0, 16, 15, 1],
                [3, 11, 0, 5, 12, 1],
                [7, 11, 0, 9, 12, 1],
                [11, 11, 0, 13, 12, 1],
            ]);
        }
        Style::Oriental => {
            // Recessed legs with hoof feet turned in, an apron with spandrels, corner tips.
            top.m(&[[0, 14, 0, 16, 16, 16]]);
            corner.m(&[
                [2, 1, 2, 4, 14, 4],
                [3, 0, 3, 5, 1, 5],
                [0, 15, 0, 1, 16, 1],
            ]);
            edge.m(&[
                [2, 12, 1, 14, 14, 2],
                [4, 11, 1, 5, 12, 2],
                [11, 11, 1, 12, 12, 2],
            ]);
        }
        Style::Root => {
            top.m(&[[0, 13, 0, 16, 16, 16]]);
            center
                .m(&[
                    [6, 0, 6, 10, 13, 10],
                    [4, 11, 6, 12, 13, 10],
                    [6, 11, 4, 10, 13, 12],
                ])
                .a(&[
                    [2, 0, 6, 6, 2, 10],
                    [10, 0, 6, 14, 2, 10],
                    [6, 0, 2, 10, 2, 6],
                    [6, 0, 10, 10, 2, 14],
                ]);
        }
        Style::Gothic => {
            // Slender legs, a pointed arcade between them.
            top.m(&[[0, 13, 0, 16, 16, 16]]);
            corner.m(&[[1, 0, 1, 3, 13, 3]]);
            edge.m(&[
                [0, 12, 0, 16, 13, 1],
                [3, 8, 0, 4, 10, 1],
                [4, 10, 0, 5, 11, 1],
                [5, 11, 0, 7, 12, 1],
                [9, 11, 0, 11, 12, 1],
                [11, 10, 0, 12, 11, 1],
                [12, 8, 0, 13, 10, 1],
            ]);
        }
        Style::Fungal => {
            top.a(&[[0, 13, 0, 16, 16, 16]]);
            center.m(&[[4, 0, 4, 12, 1, 12], [5, 1, 5, 11, 12, 11]]);
            edge.m(&[[0, 12, 1, 16, 13, 2]]);
        }
        Style::Twisted => {
            top.m(&[[0, 13, 0, 16, 16, 16]]);
            center.m(&[
                [6, 0, 5, 10, 3, 9],
                [7, 3, 6, 11, 6, 10],
                [6, 6, 7, 10, 9, 11],
                [5, 9, 6, 9, 13, 10],
                [3, 0, 3, 6, 1, 6],
                [10, 0, 10, 13, 1, 13],
            ]);
            corner.a(&[[0, 11, 0, 2, 13, 2]]);
        }
        Style::Shaker => {
            // Tapered legs, a clean apron with a drawer knob on each open side.
            top.m(&[[0, 14, 0, 16, 16, 16]]);
            corner.m(&[[1, 7, 1, 4, 14, 4], [1, 0, 1, 3, 7, 3]]);
            edge.m(&[[1, 11, 1, 15, 14, 2], [7, 12, 0, 9, 13, 1]]);
        }
        Style::Modern => {
            top.m(&[[0, 14, 0, 16, 16, 16]]);
            corner.m(&[[0, 0, 0, 2, 4, 2], [1, 4, 1, 3, 9, 3], [2, 9, 2, 4, 14, 4]]);
            edge.a(&[[0, 13, 2, 16, 14, 3]]);
        }
        Style::Colonial => {
            top.m(&[[0, 13, 0, 16, 16, 16]]);
            corner.m(&[[1, 0, 1, 3, 11, 3], [0, 3, 0, 4, 4, 4], [0, 8, 0, 4, 9, 4]]);
            edge.m(&[[0, 11, 1, 16, 13, 2], [6, 10, 1, 10, 11, 2]]);
            corner.a(&[[1, 0, 1, 3, 1, 3]]);
        }
        Style::Wicker => {
            // A woven skirt and a woven shelf low down.
            top.m(&[[0, 14, 0, 16, 16, 16]]).a(&[[0, 4, 0, 16, 5, 16]]);
            corner.m(&[[1, 0, 1, 3, 14, 3]]);
            edge.a(&[[0, 10, 0, 16, 14, 1]]);
        }
        Style::Beach => {
            // A tiki bar table: a thatch fringe round the top.
            top.m(&[[0, 14, 0, 16, 16, 16]]);
            corner.m(&[[1, 0, 1, 4, 14, 4]]);
            edge.a(&[
                [0, 11, 0, 16, 14, 1],
                [1, 10, 0, 3, 11, 1],
                [5, 10, 0, 7, 11, 1],
                [9, 10, 0, 11, 11, 1],
                [13, 10, 0, 15, 11, 1],
            ]);
        }
        Style::Deck => {
            // L-shaped panel legs at the corners, a thick plank top.
            top.m(&[[0, 13, 0, 16, 16, 16]]);
            corner.m(&[[0, 0, 0, 4, 13, 2], [0, 0, 2, 2, 13, 4]]);
        }
        Style::Rococo => {
            // Cabriole legs, a scalloped apron with a fruit cluster at its middle.
            top.m(&[[0, 13, 0, 16, 16, 16]]);
            corner.m(&[
                [0, 0, 0, 3, 1, 3],
                [1, 1, 1, 3, 5, 3],
                [1, 5, 1, 2, 8, 2],
                [0, 8, 0, 3, 13, 3],
            ]);
            edge.m(&[[0, 11, 0, 16, 13, 1], [6, 10, 0, 10, 11, 1]]).a(&[
                [7, 9, 0, 9, 10, 1],
                [6, 8, 0, 8, 9, 1],
                [8, 7, 0, 9, 8, 1],
            ]);
        }
        Style::Ancient => {
            top.m(&[[0, 13, 0, 16, 16, 16]]);
            corner
                .m(&[[0, 0, 0, 5, 2, 5], [1, 2, 1, 4, 13, 4]])
                .a(&[[2, 4, 0, 3, 10, 1], [0, 4, 2, 1, 10, 3]]);
            edge.a(&[[0, 12, 0, 16, 13, 1]]);
        }
        Style::Celestial => {
            top.m(&[[0, 13, 0, 16, 16, 16]]);
            corner
                .a(&[[0, 0, 0, 4, 2, 4], [1, 2, 1, 3, 3, 3]])
                .m(&[[1, 3, 1, 3, 13, 3]]);
            edge.a(&[[3, 12, 0, 13, 13, 1]]);
        }
    }
    TableParts {
        top,
        corner,
        edge,
        center,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn all_boxes(shape: Shape, s: Style) -> Vec<(u8, u8, Vec<PBox>, usize)> {
        let mut v = Vec::new();
        let (variants, conns): (u8, u8) = if shape == Shape::Table {
            (1, 16)
        } else {
            (4, 1)
        };
        for var in 0..variants {
            for conn in 0..conns {
                let mut out = Vec::new();
                let a = boxes(shape, s, var, conn, &mut out);
                v.push((var, conn, out, a));
            }
        }
        v
    }

    #[test]
    fn every_style_builds_valid_boxes() {
        for s in Style::ALL {
            for shape in [Shape::Chair, Shape::Table, Shape::Bench, Shape::Shelf] {
                for (var, conn, out, a) in all_boxes(shape, s) {
                    assert!(!out.is_empty() && a <= out.len(), "{s:?} {shape:?}");
                    for b in &out {
                        assert!(
                            b[0] < b[3] && b[1] < b[4] && b[2] < b[5],
                            "{s:?} {shape:?} v{var} c{conn}: empty box {b:?}"
                        );
                        assert!(
                            b[3] <= 16 && b[4] <= 16 && b[5] <= 16,
                            "{s:?} {shape:?}: box out of the block {b:?}"
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn chairs_seat_at_the_same_height_and_styles_differ() {
        let mut seen = std::collections::HashSet::new();
        for s in Style::ALL {
            let mut out = Vec::new();
            boxes(Shape::Chair, s, 0, 0, &mut out);
            // Something reaches seat height under the middle of the seat.
            let seat = out
                .iter()
                .any(|b| b[4] == 10 && b[0] <= 8 && b[3] >= 8 && b[2] <= 6 && b[5] >= 6);
            assert!(seat, "{s:?}: no seat top at 10 pixels");
            let mut key = out.clone();
            key.sort();
            assert!(seen.insert(key), "{s:?}: same chair as another style");
            for shape in [Shape::Table, Shape::Bench, Shape::Shelf] {
                let mut o = Vec::new();
                boxes(shape, s, 0, 0, &mut o);
                o.sort();
                assert!(seen.insert(o), "{s:?} {shape:?}: same as another style");
            }
        }
    }

    #[test]
    fn joined_tables_drop_inner_legs_and_aprons() {
        for s in Style::ALL {
            let (mut lone, mut joined) = (Vec::new(), Vec::new());
            boxes(Shape::Table, s, 0, 0, &mut lone);
            boxes(
                Shape::Table,
                s,
                0,
                CONN_PX | CONN_NX | CONN_PZ | CONN_NZ,
                &mut joined,
            );
            let p = table_parts(s);
            assert_eq!(
                joined.len(),
                p.top.m.len() + p.top.a.len() + p.center.m.len() + p.center.a.len(),
                "{s:?}"
            );
            assert!(lone.len() >= joined.len());
        }
    }
}
