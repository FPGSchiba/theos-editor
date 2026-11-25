//! Plain shapes for features: one `Shape` per kind of mark, drawn filled with an outline.

use tiny_skia::{Path, PathBuilder, Pixmap, Rect};

use super::canvas;
use super::palette::Rgb;
use crate::kernels::num::{to_f32, to_f64};

/// The outline width of a mark, pixels.
const EDGE_PX: f64 = 1.0;

/// The shape of a mark.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Shape {
    /// A square.
    Square,
    /// A square on its corner.
    Diamond,
    /// A triangle pointing up.
    Triangle,
    /// A triangle pointing down.
    TriangleDown,
    /// A circle.
    Circle,
    /// A five-pointed star.
    Star,
    /// A hexagon.
    Hexagon,
    /// A plus sign.
    Plus,
    /// A diagonal cross.
    Cross,
    /// A thin vertical bar.
    Bar,
    /// A thin horizontal dash.
    Dash,
}

/// A shape with a size and two colours.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Mark {
    /// The shape.
    pub shape: Shape,
    /// The width (diameter) in pixels.
    pub size: u8,
    /// The fill colour.
    pub face: Rgb,
    /// The outline colour.
    pub edge: Rgb,
}

impl Mark {
    /// The same mark at another size.
    pub(crate) fn sized(self, size: u8) -> Self {
        Self { size, ..self }
    }

    /// Draws the mark centred on `at` (pixels).
    pub(crate) fn draw(self, pixmap: &mut Pixmap, at: [f64; 2]) {
        if let Some(path) = path(self.shape, at, f64::from(self.size)) {
            canvas::fill(pixmap, &path, self.face);
            canvas::outline(pixmap, &path, EDGE_PX, self.edge);
        }
    }
}

/// A closed polygon through `points`.
fn polygon(points: &[[f64; 2]]) -> Option<Path> {
    let (first, rest) = points.split_first()?;
    let mut b = PathBuilder::new();
    b.move_to(to_f32(first[0]), to_f32(first[1]));
    for p in rest {
        b.line_to(to_f32(p[0]), to_f32(p[1]));
    }
    b.close();
    b.finish()
}

/// The corners of a rectangle `half_w` and `half_h` from `at`.
fn rectangle(at: [f64; 2], half_w: f64, half_h: f64) -> Option<Path> {
    let rect = Rect::from_xywh(
        to_f32(at[0] - half_w),
        to_f32(at[1] - half_h),
        to_f32(2.0 * half_w),
        to_f32(2.0 * half_h),
    )?;
    Some(PathBuilder::from_rect(rect))
}

/// A polygon of `n` corners on circles of `radii` (cycled) around `at`; the first corner lies
/// `start_deg` degrees clockwise from east (y points down).
fn ring(at: [f64; 2], n: usize, radii: &[f64], start_deg: f64) -> Option<Path> {
    let points: Vec<[f64; 2]> = (0..n)
        .map(|i| {
            let turn = 360.0 * to_f64(i) / to_f64(n) + start_deg;
            let (sin, cos) = libm::sincos(turn.to_radians());
            let r = radii[i % radii.len()];
            [at[0] + r * cos, at[1] + r * sin]
        })
        .collect();
    polygon(&points)
}

/// A cross of arms `half` long and `arm` wide, turned by `deg`.
fn cross(at: [f64; 2], half: f64, arm: f64, deg: f64) -> Option<Path> {
    let (sin, cos) = libm::sincos(deg.to_radians());
    let a = arm / 2.0;
    let outline = [
        [-a, -half],
        [a, -half],
        [a, -a],
        [half, -a],
        [half, a],
        [a, a],
        [a, half],
        [-a, half],
        [-a, a],
        [-half, a],
        [-half, -a],
        [-a, -a],
    ];
    let points: Vec<[f64; 2]> = outline
        .iter()
        .map(|p| {
            [
                at[0] + p[0] * cos - p[1] * sin,
                at[1] + p[0] * sin + p[1] * cos,
            ]
        })
        .collect();
    polygon(&points)
}

/// The outline of `shape` of width `size` centred on `at` (pixels).
pub(crate) fn path(shape: Shape, at: [f64; 2], size: f64) -> Option<Path> {
    let r = size / 2.0;
    match shape {
        Shape::Square => rectangle(at, r, r),
        Shape::Diamond => ring(at, 4, &[r * 1.2], -90.0),
        Shape::Triangle => ring(at, 3, &[r * 1.15], -90.0),
        Shape::TriangleDown => ring(at, 3, &[r * 1.15], 90.0),
        Shape::Circle => PathBuilder::from_circle(to_f32(at[0]), to_f32(at[1]), to_f32(r)),
        Shape::Star => ring(at, 10, &[r * 1.3, r * 0.55], -90.0),
        Shape::Hexagon => ring(at, 6, &[r], 0.0),
        Shape::Plus => cross(at, r, size / 3.0, 0.0),
        Shape::Cross => cross(at, r, size / 3.0, 45.0),
        Shape::Bar => rectangle(at, size / 8.0, r * 1.2),
        Shape::Dash => rectangle(at, r * 1.2, size / 8.0),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::test_support::decode;

    const ALL: [Shape; 11] = [
        Shape::Square,
        Shape::Diamond,
        Shape::Triangle,
        Shape::TriangleDown,
        Shape::Circle,
        Shape::Star,
        Shape::Hexagon,
        Shape::Plus,
        Shape::Cross,
        Shape::Bar,
        Shape::Dash,
    ];

    #[test]
    fn every_shape_draws_its_face_at_its_centre() {
        let mark = |shape| Mark {
            shape,
            size: 12,
            face: [200, 10, 10],
            edge: [0, 0, 0],
        };
        for shape in ALL {
            let mut pixmap = Pixmap::new(32, 32).unwrap();
            mark(shape).draw(&mut pixmap, [16.0, 16.0]);
            let png = canvas::encode_png(&pixmap).unwrap();
            let image = decode(&png);
            let centre = image.rgba(16, 16);
            let inked = (0..32).any(|x| (0..32).any(|y| image.rgba(x, y)[3] > 0));
            assert!(inked, "{shape:?} draws something");
            // Plus and Cross have their arms through the centre; a bar or dash is thin but
            // centred: the centre pixel is the face colour for every shape.
            assert_eq!(centre[..3], [200, 10, 10], "{shape:?}");
        }
        assert_eq!(mark(Shape::Star).sized(20).size, 20);
    }
}
