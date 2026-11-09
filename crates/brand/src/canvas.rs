use std::f32::consts::TAU;

/// An sRGB colour.
pub type Rgb = [u8; 3];

/// A point in drawing units, y pointing down.
pub type Point = (f32, f32);

/// A square straight-alpha RGBA image, rows from the top.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Image {
    pub size: u32,
    pub rgba: Vec<u8>,
}

impl Image {
    /// The RGBA bytes of the pixel at column `x`, row `y`.
    pub fn pixel(&self, x: u32, y: u32) -> [u8; 4] {
        let at = ((y * self.size + x) * 4) as usize;
        [
            self.rgba[at],
            self.rgba[at + 1],
            self.rgba[at + 2],
            self.rgba[at + 3],
        ]
    }
}

/// A filled region of the drawing.
pub enum Shape {
    /// A polygon, grown by `grow` units on every side with round joins.
    Poly {
        points: Vec<Point>,
        grow: f32,
    },
    /// A segment `half` units either side of its axis, with round caps.
    Line {
        from: Point,
        to: Point,
        half: f32,
    },
    Circle {
        centre: Point,
        radius: f32,
    },
    /// A circle's outline `half` units either side; `dash` is (on, off) along it, from 3 o'clock clockwise.
    Ring {
        centre: Point,
        radius: f32,
        half: f32,
        dash: Option<(f32, f32)>,
    },
    Rect {
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        radius: f32,
    },
    /// Everything any of its shapes covers, painted once.
    Union(Vec<Shape>),
}

impl Shape {
    fn covers(&self, x: f32, y: f32) -> bool {
        match self {
            Shape::Poly { points, grow } => {
                inside(points, x, y)
                    || (*grow > 0.0 && edges(points).any(|(a, b)| distance(x, y, a, b) <= *grow))
            }
            Shape::Line { from, to, half } => distance(x, y, *from, *to) <= *half,
            Shape::Circle { centre, radius } => (x - centre.0).hypot(y - centre.1) <= *radius,
            Shape::Ring {
                centre,
                radius,
                half,
                dash,
            } => {
                let (dx, dy) = (x - centre.0, y - centre.1);
                if (dx.hypot(dy) - radius).abs() > *half {
                    return false;
                }
                dash.is_none_or(|(on, off)| {
                    let along = dy.atan2(dx).rem_euclid(TAU) * radius;
                    along % (on + off) < on
                })
            }
            Shape::Rect {
                x: left,
                y: top,
                w,
                h,
                radius,
            } => {
                let (dx, dy) = ((x - left - w / 2.0).abs(), (y - top - h / 2.0).abs());
                if dx > w / 2.0 || dy > h / 2.0 {
                    return false;
                }
                let (cx, cy) = (dx - (w / 2.0 - radius), dy - (h / 2.0 - radius));
                cx <= 0.0 || cy <= 0.0 || cx.hypot(cy) <= *radius
            }
            Shape::Union(shapes) => shapes.iter().any(|shape| shape.covers(x, y)),
        }
    }

    /// The box around the shape: left, top, right, bottom.
    fn bounds(&self) -> [f32; 4] {
        match self {
            Shape::Poly { points, grow } => around(points.iter().copied(), *grow),
            Shape::Line { from, to, half } => around([*from, *to], *half),
            Shape::Circle { centre, radius } => around([*centre], *radius),
            Shape::Ring {
                centre,
                radius,
                half,
                ..
            } => around([*centre], radius + half),
            Shape::Rect { x, y, w, h, .. } => [*x, *y, x + w, y + h],
            Shape::Union(shapes) => around(
                shapes.iter().flat_map(|shape| {
                    let [l, t, r, b] = shape.bounds();
                    [(l, t), (r, b)]
                }),
                0.0,
            ),
        }
    }
}

/// The box around `points`, `pad` units wider on every side.
fn around(points: impl IntoIterator<Item = Point>, pad: f32) -> [f32; 4] {
    points.into_iter().fold(
        [f32::MAX, f32::MAX, f32::MIN, f32::MIN],
        |[l, t, r, b], (x, y)| {
            [
                l.min(x - pad),
                t.min(y - pad),
                r.max(x + pad),
                b.max(y + pad),
            ]
        },
    )
}

/// Even-odd test of a point against a closed polygon.
fn inside(points: &[Point], x: f32, y: f32) -> bool {
    edges(points)
        .filter(|&((ax, ay), (bx, by))| {
            (ay > y) != (by > y) && x < ax + (y - ay) / (by - ay) * (bx - ax)
        })
        .count()
        % 2
        == 1
}

fn edges(points: &[Point]) -> impl Iterator<Item = (Point, Point)> + '_ {
    points
        .iter()
        .copied()
        .zip(points.iter().copied().cycle().skip(1))
}

/// Distance from a point to the segment `a`–`b`.
fn distance(x: f32, y: f32, a: Point, b: Point) -> f32 {
    let (vx, vy) = (b.0 - a.0, b.1 - a.1);
    let length = vx * vx + vy * vy;
    let t = if length == 0.0 {
        0.0
    } else {
        (((x - a.0) * vx + (y - a.1) * vy) / length).clamp(0.0, 1.0)
    };
    (x - a.0 - t * vx).hypot(y - a.1 - t * vy)
}

/// A drawing surface of `size` pixels a side over `grid` drawing units, premultiplied.
pub struct Canvas {
    size: u32,
    unit: f32,
    pixels: Vec<[f32; 4]>,
}

impl Canvas {
    pub fn new(size: u32, grid: f32) -> Canvas {
        Canvas {
            size,
            unit: grid / size as f32,
            pixels: vec![[0.0; 4]; (size * size) as usize],
        }
    }

    /// Paints `shape` over what is there, at `opacity`.
    pub fn fill(&mut self, shape: &Shape, colour: Rgb, opacity: f32) {
        let coverage = self.coverage(shape);
        self.paint(&coverage, colour, opacity);
    }

    /// Clears what `shape` covers back to transparent.
    pub fn erase(&mut self, shape: &Shape) {
        let coverage = self.coverage(shape);
        for (pixel, cover) in self.pixels.iter_mut().zip(coverage) {
            pixel.iter_mut().for_each(|channel| *channel *= 1.0 - cover);
        }
    }

    /// Paints the blurred shape in black at `opacity`; `sigma` is in drawing units.
    pub fn shadow(&mut self, shape: &Shape, sigma: f32, opacity: f32) {
        let blurred = blur(&self.coverage(shape), self.size as usize, sigma / self.unit);
        self.paint(&blurred, [0, 0, 0], opacity);
    }

    pub fn image(&self) -> Image {
        let byte = |value: f32| (value.clamp(0.0, 1.0) * 255.0).round() as u8;
        let rgba = self
            .pixels
            .iter()
            .flat_map(|&[r, g, b, a]| {
                if a <= 0.0 {
                    [0; 4]
                } else {
                    [byte(r / a), byte(g / a), byte(b / a), byte(a)]
                }
            })
            .collect();
        Image {
            size: self.size,
            rgba,
        }
    }

    fn paint(&mut self, coverage: &[f32], colour: Rgb, opacity: f32) {
        let [r, g, b] = colour.map(|channel| f32::from(channel) / 255.0);
        for (pixel, cover) in self.pixels.iter_mut().zip(coverage) {
            let a = cover * opacity;
            let [pr, pg, pb, pa] = *pixel;
            *pixel = [
                r * a + pr * (1.0 - a),
                g * a + pg * (1.0 - a),
                b * a + pb * (1.0 - a),
                a + pa * (1.0 - a),
            ];
        }
    }

    /// The share of each pixel the shape covers, from a grid of samples inside the pixel.
    fn coverage(&self, shape: &Shape) -> Vec<f32> {
        let size = self.size as usize;
        let samples = if self.size <= 64 { 16 } else { 8 };
        let mut coverage = vec![0.0; size * size];
        let [left, top, right, bottom] = shape.bounds();
        let span = |from: f32, to: f32| {
            let first = (from / self.unit).floor().max(0.0) as usize;
            let last = ((to / self.unit).ceil().max(0.0) as usize).min(size);
            first..last
        };
        for row in span(top, bottom) {
            for column in span(left, right) {
                let mut hits = 0;
                for j in 0..samples {
                    let y = (row as f32 + (j as f32 + 0.5) / samples as f32) * self.unit;
                    for i in 0..samples {
                        let x = (column as f32 + (i as f32 + 0.5) / samples as f32) * self.unit;
                        hits += usize::from(shape.covers(x, y));
                    }
                }
                coverage[row * size + column] = hits as f32 / (samples * samples) as f32;
            }
        }
        coverage
    }
}

/// A Gaussian blur of a square mask, `sigma` in pixels; outside the mask counts as empty.
fn blur(mask: &[f32], size: usize, sigma: f32) -> Vec<f32> {
    let reach = (sigma * 3.0).ceil() as isize;
    let weights: Vec<f32> = (-reach..=reach)
        .map(|d| (-(d * d) as f32 / (2.0 * sigma * sigma)).exp())
        .collect();
    let total: f32 = weights.iter().sum();
    let pass = |source: &[f32], step: (usize, usize)| {
        let mut out = vec![0.0; size * size];
        for row in 0..size {
            for column in 0..size {
                let mut sum = 0.0;
                for (k, weight) in weights.iter().enumerate() {
                    let offset = k as isize - reach;
                    let (r, c) = (
                        row as isize + offset * step.0 as isize,
                        column as isize + offset * step.1 as isize,
                    );
                    if (0..size as isize).contains(&r) && (0..size as isize).contains(&c) {
                        sum += weight * source[r as usize * size + c as usize];
                    }
                }
                out[row * size + column] = sum / total;
            }
        }
        out
    };
    pass(&pass(mask, (0, 1)), (1, 0))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_full_pixel_is_opaque_and_an_edge_is_partial() {
        let mut canvas = Canvas::new(4, 4.0);
        let square = Shape::Rect {
            x: 0.0,
            y: 0.0,
            w: 2.5,
            h: 4.0,
            radius: 0.0,
        };
        canvas.fill(&square, [10, 20, 30], 1.0);
        let image = canvas.image();

        assert_eq!(image.pixel(0, 0), [10, 20, 30, 255]);
        assert_eq!(image.pixel(2, 0)[3], 128);
        assert_eq!(image.pixel(3, 0), [0; 4]);
    }

    #[test]
    fn erasing_clears_back_to_transparent() {
        let mut canvas = Canvas::new(4, 4.0);
        let all = Shape::Rect {
            x: 0.0,
            y: 0.0,
            w: 4.0,
            h: 4.0,
            radius: 0.0,
        };
        canvas.fill(&all, [255, 255, 255], 1.0);
        canvas.erase(&Shape::Circle {
            centre: (2.0, 2.0),
            radius: 0.5,
        });
        let image = canvas.image();

        assert!(image.pixel(1, 1)[3] < 255);
        assert_eq!(image.pixel(0, 0)[3], 255);
    }

    #[test]
    fn a_dashed_ring_has_gaps() {
        let solid = Shape::Ring {
            centre: (0.0, 0.0),
            radius: 2.0,
            half: 0.5,
            dash: None,
        };
        let dashed = Shape::Ring {
            centre: (0.0, 0.0),
            radius: 2.0,
            half: 0.5,
            dash: Some((1.0, 1.0)),
        };
        // 1.5 units along the ring from 3 o'clock falls in the first gap.
        let gap = (2.0 * 0.75_f32.cos(), 2.0 * 0.75_f32.sin());

        assert!(solid.covers(gap.0, gap.1));
        assert!(!dashed.covers(gap.0, gap.1));
        assert!(dashed.covers(2.0, 0.1));
    }

    #[test]
    fn a_polygon_grows_by_its_margin() {
        let triangle = Shape::Poly {
            points: vec![(0.0, 0.0), (4.0, 0.0), (0.0, 4.0)],
            grow: 0.5,
        };

        assert!(triangle.covers(1.0, 1.0));
        assert!(triangle.covers(2.0, -0.4));
        assert!(!triangle.covers(2.0, -0.6));
    }

    #[test]
    fn blur_keeps_the_mass_away_from_the_edges() {
        let mut mask = vec![0.0; 21 * 21];
        mask[10 * 21 + 10] = 1.0;
        let blurred = blur(&mask, 21, 1.5);

        assert!((blurred.iter().sum::<f32>() - 1.0).abs() < 1e-4);
        assert!(blurred[10 * 21 + 10] < 1.0);
        assert!(blurred[10 * 21 + 12] > 0.0);
    }
}
