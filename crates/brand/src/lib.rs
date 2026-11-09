//! The Lapidado mark of Vestigium, a V cut like a gemstone, drawn as pixels.

mod canvas;

use canvas::{Canvas, Point, Shape};
pub use canvas::{Image, Rgb};

/// How much the drawings below are enlarged around their centre, so the V spans 94% of the icon.
const GROW_48: f32 = 1.32;
const GROW_16: f32 = 1.3;

/// The V outline on the 48 grid.
const OUTLINE: [Point; 6] = [
    (7.0, 9.0),
    (17.0, 9.0),
    (24.0, 22.5),
    (31.0, 9.0),
    (41.0, 9.0),
    (24.0, 40.0),
];

/// The six facets on the 48 grid, from the lit left one to the shadowed right one.
const FACETS: [[Point; 3]; 6] = [
    [(7.0, 9.0), (12.0, 9.0), (24.0, 40.0)],
    [(12.0, 9.0), (17.0, 9.0), (24.0, 40.0)],
    [(17.0, 9.0), (24.0, 22.5), (24.0, 40.0)],
    [(31.0, 9.0), (24.0, 22.5), (24.0, 40.0)],
    [(36.0, 9.0), (31.0, 9.0), (24.0, 40.0)],
    [(41.0, 9.0), (36.0, 9.0), (24.0, 40.0)],
];

/// The edges between facets on the 48 grid.
const EDGES: [(Point, Point); 5] = [
    ((12.0, 9.0), (24.0, 40.0)),
    ((17.0, 9.0), (24.0, 40.0)),
    ((31.0, 9.0), (24.0, 40.0)),
    ((36.0, 9.0), (24.0, 40.0)),
    ((24.0, 22.5), (24.0, 40.0)),
];

/// The four facets of the 16 px drawing.
const FACETS_16: [&[Point]; 4] = [
    &[(2.2, 3.0), (3.9, 3.0), (8.0, 13.2)],
    &[(3.9, 3.0), (5.6, 3.0), (8.0, 7.6), (8.0, 13.2)],
    &[(10.4, 3.0), (12.1, 3.0), (8.0, 13.2), (8.0, 7.6)],
    &[(12.1, 3.0), (13.8, 3.0), (8.0, 13.2)],
];

/// The fixed facet colours of the app icon, facet-1 to facet-6; the 16 px drawing takes 1, 2, 4 and 6.
const FACET_COLOURS: [Rgb; 6] = [
    [0xff, 0x53, 0x55],
    [0xff, 0xa0, 0x9a],
    [0xf5, 0x1c, 0x35],
    [0xc5, 0x01, 0x24],
    [0xe3, 0x0e, 0x2d],
    [0x9b, 0x0e, 0x1e],
];

/// What the tray badge shows; each state has its own shape.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Badge {
    #[default]
    None,
    Attention,
    Problem,
    Paused,
    Sensitive,
}

/// The colour themes of the app; the tray takes each one's accent.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Scheme {
    #[default]
    Red,
    Graphite,
    Slate,
    Indigo,
}

/// The colours of the tray glyph: the taskbar's ink and the theme's accent on that taskbar.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TrayInk {
    pub ink: Rgb,
    pub accent: Rgb,
}

impl TrayInk {
    pub fn of(scheme: Scheme, dark_taskbar: bool) -> TrayInk {
        let accent = match (scheme, dark_taskbar) {
            (Scheme::Red, true) => [0xfe, 0x8a, 0x85],
            (Scheme::Red, false) => [0x9a, 0x01, 0x1a],
            (Scheme::Graphite, true) => [0xf2, 0xf2, 0xf2],
            (Scheme::Graphite, false) => [0x14, 0x14, 0x14],
            (Scheme::Slate, true) => [0xb0, 0xa5, 0xf2],
            (Scheme::Slate, false) => [0x4f, 0x43, 0x8c],
            (Scheme::Indigo, true) => [0x9c, 0xaa, 0xfc],
            (Scheme::Indigo, false) => [0x3d, 0x42, 0xa3],
        };
        let ink = if dark_taskbar {
            [0xff, 0xff, 0xff]
        } else {
            [0x1b, 0x1b, 0x1b]
        };
        TrayInk { ink, accent }
    }
}

/// The tray glyph, `size` pixels a side: the app icon's 16 px facets, its badge in the bottom-right quadrant.
pub fn tray(size: u32, badge: Badge, colours: TrayInk) -> Image {
    let mut canvas = Canvas::new(size, 16.0);
    let faded = if badge == Badge::Paused { 0.45 } else { 1.0 };
    facets_16(&mut canvas, faded);

    let centre = (12.5, 12.5);
    let knockout = |radius| Shape::Circle { centre, radius };
    match badge {
        Badge::None => {}
        Badge::Attention => {
            canvas.erase(&knockout(3.8));
            let ring = Shape::Ring {
                centre,
                radius: 2.3,
                half: 0.7,
                dash: None,
            };
            canvas.fill(&ring, colours.accent, 1.0);
        }
        Badge::Problem => {
            canvas.erase(&knockout(3.9));
            canvas.fill(&knockout(2.9), colours.accent, 1.0);
        }
        Badge::Paused => {
            canvas.erase(&Shape::Rect {
                x: 8.6,
                y: 8.6,
                w: 7.4,
                h: 7.4,
                radius: 2.0,
            });
            for x in [10.0, 13.0] {
                let bar = Shape::Rect {
                    x,
                    y: 10.0,
                    w: 1.6,
                    h: 4.6,
                    radius: 0.6,
                };
                canvas.fill(&bar, colours.ink, 1.0);
            }
        }
        Badge::Sensitive => {
            canvas.erase(&knockout(3.9));
            let ring = Shape::Ring {
                centre,
                radius: 2.6,
                half: 0.55,
                dash: Some((1.4, 1.0)),
            };
            canvas.fill(&ring, colours.ink, 1.0);
            canvas.fill(&knockout(0.9), colours.ink, 1.0);
        }
    }
    canvas.image()
}

/// The app icon, `size` pixels a side: the red Lapidado with its fixed facets, in every theme.
pub fn app_icon(size: u32) -> Image {
    if size <= 16 {
        let mut canvas = Canvas::new(size, 16.0);
        facets_16(&mut canvas, 1.0);
        return canvas.image();
    }
    let mut canvas = Canvas::new(size, 48.0);
    let lowered = Shape::Poly {
        points: OUTLINE.map(grow_48).map(|(x, y)| (x, y + 1.2)).to_vec(),
        grow: 0.3,
    };
    canvas.shadow(&lowered, 1.2, 0.35);
    for (points, colour) in FACETS.iter().zip(FACET_COLOURS) {
        let facet = Shape::Poly {
            points: points.map(grow_48).to_vec(),
            grow: 0.3,
        };
        canvas.fill(&facet, colour, 1.0);
    }
    let edges = EDGES.map(|(from, to)| Shape::Line {
        from: grow_48(from),
        to: grow_48(to),
        half: 0.35,
    });
    canvas.fill(&Shape::Union(edges.into()), [0xff; 3], 0.22);
    canvas.image()
}

/// The four facets of the 16 px drawing, at `alpha`.
fn facets_16(canvas: &mut Canvas, alpha: f32) {
    let colours = [0, 1, 3, 5].map(|facet| FACET_COLOURS[facet]);
    for (points, colour) in FACETS_16.iter().zip(colours) {
        let facet = Shape::Poly {
            points: points.iter().map(|&p| grow_16(p)).collect(),
            grow: 0.15,
        };
        canvas.fill(&facet, colour, alpha);
    }
}

fn grow_48((x, y): Point) -> Point {
    (24.0 + (x - 24.0) * GROW_48, 24.5 + (y - 24.5) * GROW_48)
}

fn grow_16((x, y): Point) -> Point {
    (8.0 + (x - 8.0) * GROW_16, 8.1 + (y - 8.1) * GROW_16)
}

#[cfg(test)]
mod tests {
    use super::*;

    const ALL: [Badge; 5] = [
        Badge::None,
        Badge::Attention,
        Badge::Problem,
        Badge::Paused,
        Badge::Sensitive,
    ];

    fn colours() -> TrayInk {
        TrayInk::of(Scheme::Red, true)
    }

    /// The alpha of the bottom-right quadrant, where the badge sits.
    fn badge_mask(image: &Image) -> Vec<u8> {
        let half = image.size / 2;
        (half..image.size)
            .flat_map(|y| (half..image.size).map(move |x| (x, y)))
            .map(|(x, y)| image.pixel(x, y)[3])
            .collect()
    }

    #[test]
    fn the_glyph_has_the_size_asked_for() {
        for size in [16, 20, 24, 32] {
            let image = tray(size, Badge::Problem, colours());
            assert_eq!(image.size, size);
            assert_eq!(image.rgba.len(), (size * size * 4) as usize);
        }
    }

    #[test]
    fn the_glyph_takes_the_app_facets_and_the_corners_stay_clear() {
        let image = tray(16, Badge::None, colours());

        assert_eq!(image.pixel(2, 3), [0xff, 0x53, 0x55, 0xff]);
        assert_eq!(image.pixel(0, 0)[3], 0);
        assert_eq!(image.pixel(15, 15)[3], 0);
        assert_eq!(image.pixel(8, 2)[3], 0);
    }

    #[test]
    fn each_state_has_its_own_shape() {
        // Compared without colour: the shapes alone must tell the states apart.
        let masks: Vec<Vec<u8>> = ALL
            .iter()
            .map(|&badge| badge_mask(&tray(16, badge, colours())))
            .collect();
        for (i, a) in masks.iter().enumerate() {
            for b in &masks[i + 1..] {
                assert_ne!(a, b);
            }
        }
    }

    #[test]
    fn the_ring_and_the_dot_take_the_accent() {
        let accent = colours().accent;
        let problem = tray(16, Badge::Problem, colours());
        let attention = tray(16, Badge::Attention, colours());

        assert_eq!(
            problem.pixel(12, 12),
            [accent[0], accent[1], accent[2], 0xff]
        );
        assert!(attention.pixel(12, 12)[3] < 0x80);
        assert_eq!(&attention.pixel(12, 10)[..3], &accent);
    }

    #[test]
    fn paused_dims_the_glyph_and_draws_bars_in_ink() {
        let image = tray(16, Badge::Paused, colours());

        assert_eq!(image.pixel(2, 3)[3], 115);
        assert_eq!(image.pixel(10, 12), [0xff, 0xff, 0xff, 0xff]);
        assert_eq!(image.pixel(12, 12)[3], 0);
    }

    #[test]
    fn sensitive_mode_draws_a_dot_inside_a_dashed_ring() {
        let image = tray(32, Badge::Sensitive, colours());
        let ring: Vec<u8> = (0..64)
            .map(|step| {
                let angle = step as f32 / 64.0 * std::f32::consts::TAU;
                let (x, y) = (25.0 + 5.2 * angle.cos(), 25.0 + 5.2 * angle.sin());
                image.pixel(x as u32, y as u32)[3]
            })
            .collect();

        assert_eq!(image.pixel(25, 25)[3], 0xff);
        assert!(ring.iter().any(|&alpha| alpha > 0xe0));
        assert!(ring.iter().any(|&alpha| alpha < 0x20));
    }

    #[test]
    fn the_app_icon_keeps_its_fixed_facets() {
        let image = app_icon(256);
        // The middle of the first and the last facet, enlarged on the 48 grid and scaled to 256.
        assert_eq!(image.pixel(60, 94), [0xff, 0x53, 0x55, 0xff]);
        assert_eq!(image.pixel(196, 94), [0x9b, 0x0e, 0x1e, 0xff]);
        assert_eq!(image.pixel(0, 0)[3], 0);
    }

    #[test]
    fn the_app_icon_casts_a_soft_shadow_below_the_tip() {
        let image = app_icon(256);
        let below = image.pixel(128, 244);

        assert_eq!(&below[..3], &[0, 0, 0]);
        assert!((1..0x80).contains(&below[3]));
    }

    #[test]
    fn the_16_px_app_icon_uses_the_four_facet_drawing() {
        let image = app_icon(16);

        assert_eq!(image.size, 16);
        assert_eq!(image.pixel(5, 4), [0xff, 0xa0, 0x9a, 0xff]);
        assert_eq!(image.pixel(8, 2)[3], 0);
    }

    #[test]
    fn a_light_taskbar_gets_dark_ink_and_the_light_accent() {
        let light = TrayInk::of(Scheme::Slate, false);

        assert_eq!(light.ink, [0x1b, 0x1b, 0x1b]);
        assert_eq!(light.accent, [0x4f, 0x43, 0x8c]);
    }
}
