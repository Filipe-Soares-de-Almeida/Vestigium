//! The Lapidado mark of Vestigium, a V cut like a gemstone, drawn as pixels.

mod canvas;

use canvas::{Canvas, Point, Shape};
pub use canvas::{Image, Rgb};

/// The V outline of the hand-tuned 16 px drawing.
const OUTLINE_16: [Point; 6] = [
    (2.2, 3.0),
    (5.6, 3.0),
    (8.0, 7.6),
    (10.4, 3.0),
    (13.8, 3.0),
    (8.0, 13.2),
];

/// The facet line the 16 px silhouette cuts, from the notch to the tip.
const CUT_16: (Point, Point) = ((8.0, 7.6), (8.0, 13.2));

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

/// The tray glyph, `size` pixels a side: the 16 px silhouette, its badge in the bottom-right quadrant.
pub fn tray(size: u32, badge: Badge, colours: TrayInk) -> Image {
    let mut canvas = Canvas::new(size, 16.0);
    let glyph = Shape::Poly {
        points: OUTLINE_16.to_vec(),
        grow: 0.25,
    };
    let faded = if badge == Badge::Paused { 0.45 } else { 1.0 };
    canvas.fill(&glyph, colours.ink, faded);
    canvas.erase(&Shape::Line {
        from: CUT_16.0,
        to: CUT_16.1,
        half: 0.3,
    });

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
    fn the_silhouette_is_ink_and_the_corners_stay_clear() {
        let image = tray(16, Badge::None, colours());

        assert_eq!(image.pixel(3, 3), [0xff, 0xff, 0xff, 0xff]);
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

        assert_eq!(image.pixel(3, 3)[3], 115);
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
    fn a_light_taskbar_gets_dark_ink_and_the_light_accent() {
        let light = TrayInk::of(Scheme::Slate, false);

        assert_eq!(light.ink, [0x1b, 0x1b, 0x1b]);
        assert_eq!(light.accent, [0x4f, 0x43, 0x8c]);
    }
}
