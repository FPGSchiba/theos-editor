//! The frame-exact pixel grid of the renders and their PNG encoding.
//!
//! A frame of `rows` x `cols` cells is `2·rows` x `2·cols` square pixels; the pixel `(x, y)`
//! covers metres `[x, x + 1) · cell_m / 2` east and `[y, y + 1) · cell_m / 2` south (G19), so a
//! point in metres lands on the pixel `floor(m / (cell_m / 2))`. Cell rasters are sampled at
//! pixel centres by bilinear interpolation between cell centres, which at two pixels per cell
//! always takes the weights 3/4 and 1/4: exact in binary floating point.
//!
//! This module is the only one that draws through tiny-skia, and it keeps every coordinate and
//! stroke width it hands over within [`Bounds`] of the pixmap: tiny-skia rasterises in `f32` and
//! fixed point, and a stroke tens of billions of pixels wide (a river of `width_a = 1e12`) fails
//! an assertion inside it (final review round 4, finding 2).

use std::borrow::Cow;

use tiny_skia::{
    FillRule, LineCap, LineJoin, Paint, Path, PathBuilder, PathSegment, Pixmap, Stroke, StrokeDash,
    Transform,
};

use super::RenderError;
use super::palette::Rgb;
use crate::grid::Grid;
use crate::kernels::num::{to_f32, to_usize};
use crate::world::MapFrame;

/// Pixels per cell edge of the frame-exact renders (spec §5.11).
pub(crate) const PX_PER_CELL: usize = 2;

/// Bytes per pixel of a pixmap (premultiplied RGBA).
const BYTES_PER_PX: usize = 4;

/// The weight of the nearer cell centre when sampling at a pixel centre.
const NEAR: f64 = 0.75;
/// The weight of the farther cell centre.
const FAR: f64 = 0.25;

/// How far beyond the pixmap a coordinate may lie, in pixmap extents (width plus height, which
/// is at least the diagonal).
const REACH: f32 = 1.0;
/// The widest stroke, in pixmap extents: twice the farthest a point inside [`Bounds`] lies from
/// a pixel of the pixmap, so a stroke this wide covers every pixel a wider one would cover.
const WIDEST: f32 = 4.0;

/// The box every coordinate handed to tiny-skia keeps, and the widest stroke.
///
/// A coordinate outside the box is moved onto its edge and a wider stroke is drawn at the widest:
/// both lie far enough outside the pixmap that a drawing within them is unchanged, and anything
/// beyond them is clamped (final review round 4, ruling on render clamping). NaN passes through,
/// and tiny-skia draws nothing for it.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Bounds {
    /// The smallest x, pixels.
    left: f32,
    /// The smallest y, pixels.
    top: f32,
    /// The largest x, pixels.
    right: f32,
    /// The largest y, pixels.
    bottom: f32,
    /// The widest stroke, pixels.
    widest: f32,
}

impl Bounds {
    /// The bounds of `pixmap`.
    fn of(pixmap: &Pixmap) -> Self {
        let (width, height) = (
            to_f32(f64::from(pixmap.width())),
            to_f32(f64::from(pixmap.height())),
        );
        let extent = width + height;
        Self {
            left: -REACH * extent,
            top: -REACH * extent,
            right: width + REACH * extent,
            bottom: height + REACH * extent,
            widest: WIDEST * extent,
        }
    }

    /// `p` (pixels) inside the box.
    fn point(self, p: [f64; 2]) -> (f32, f32) {
        (
            clamp(to_f32(p[0]), self.left, self.right),
            clamp(to_f32(p[1]), self.top, self.bottom),
        )
    }

    /// The stroke width `width_px`, at most [`Bounds::widest`].
    fn width(self, width_px: f64) -> f32 {
        let width = to_f32(width_px);
        if width > self.widest {
            self.widest
        } else {
            width
        }
    }

    /// `path` with every point inside the box: `path` itself when it already is, else a copy of
    /// it with its points clamped; `None` when the copy is no path.
    fn path(self, path: &Path) -> Option<Cow<'_, Path>> {
        let rect = path.bounds();
        let inside = rect.left() >= self.left
            && rect.top() >= self.top
            && rect.right() <= self.right
            && rect.bottom() <= self.bottom;
        if inside {
            return Some(Cow::Borrowed(path));
        }
        let at = |point: tiny_skia::Point| {
            (
                clamp(point.x, self.left, self.right),
                clamp(point.y, self.top, self.bottom),
            )
        };
        let mut builder = PathBuilder::new();
        for segment in path.segments() {
            match segment {
                PathSegment::MoveTo(end) => {
                    let (x, y) = at(end);
                    builder.move_to(x, y);
                }
                PathSegment::LineTo(end) => {
                    let (x, y) = at(end);
                    builder.line_to(x, y);
                }
                PathSegment::QuadTo(control, end) => {
                    let (control, (x, y)) = (at(control), at(end));
                    builder.quad_to(control.0, control.1, x, y);
                }
                PathSegment::CubicTo(first, second, end) => {
                    let (first, second, (x, y)) = (at(first), at(second), at(end));
                    builder.cubic_to(first.0, first.1, second.0, second.1, x, y);
                }
                PathSegment::Close => builder.close(),
            }
        }
        builder.finish().map(Cow::Owned)
    }
}

/// `x` moved into `[lo, hi]`; `x` itself, bit for bit, when it lies inside, and NaN stays NaN.
fn clamp(x: f32, lo: f32, hi: f32) -> f32 {
    if x < lo {
        lo
    } else if x > hi {
        hi
    } else {
        x
    }
}

/// A transparent pixmap of `frame` at [`PX_PER_CELL`].
pub(crate) fn pixmap(frame: &MapFrame) -> Result<Pixmap, RenderError> {
    let size = RenderError::Size {
        rows: frame.rows,
        cols: frame.cols,
    };
    let px = |cells: usize| {
        cells
            .checked_mul(PX_PER_CELL)
            .and_then(|n| u32::try_from(n).ok())
    };
    let (Some(width), Some(height)) = (px(frame.cols), px(frame.rows)) else {
        return Err(size);
    };
    Pixmap::new(width, height).ok_or(size)
}

/// Metres per pixel edge of `frame` at [`PX_PER_CELL`].
pub(crate) fn metres_per_px(frame: &MapFrame) -> f64 {
    frame.cell_m / 2.0
}

/// A value that can be interpolated between cell centres.
pub(crate) trait Lerp: Copy {
    /// `self·wa + other·wb`.
    fn blend(self, wa: f64, other: Self, wb: f64) -> Self;
}

impl Lerp for f64 {
    fn blend(self, wa: f64, other: Self, wb: f64) -> Self {
        self * wa + other * wb
    }
}

impl Lerp for [f64; 3] {
    fn blend(self, wa: f64, other: Self, wb: f64) -> Self {
        [0, 1, 2].map(|i| self[i] * wa + other[i] * wb)
    }
}

/// The nearer and the farther cell of pixel `p` along an axis of `n` cells: pixel `p` lies a
/// quarter cell from the centre of cell `p / 2`, towards the previous cell for even `p` and the
/// next for odd `p`. At the edges both are the edge cell.
fn taps(p: usize, n: usize) -> (usize, usize) {
    let near = (p / PX_PER_CELL).min(n.saturating_sub(1));
    let far = if p.is_multiple_of(PX_PER_CELL) {
        near.saturating_sub(1)
    } else {
        (near + 1).min(n.saturating_sub(1))
    };
    (near, far)
}

/// `grid` at the centre of the pixel in row `y`, column `x`, interpolated bilinearly between the
/// four nearest cell centres.
pub(crate) fn sample<T: Lerp>(grid: &Grid<T>, y: usize, x: usize) -> T {
    let (r0, r1) = taps(y, grid.rows());
    let (c0, c1) = taps(x, grid.cols());
    let row = |r| grid.get(r, c0).blend(NEAR, grid.get(r, c1), FAR);
    row(r0).blend(NEAR, row(r1), FAR)
}

/// A pixel of a pixmap's data: premultiplied red, green, blue and alpha.
pub(crate) type Px = [u8; BYTES_PER_PX];

/// Writes `colour`, opaque, into the pixel `px`.
pub(crate) fn put(px: &mut Px, colour: Rgb) {
    *px = [colour[0], colour[1], colour[2], u8::MAX];
}

/// The rows of `pixmap`, top to bottom, as pixels.
pub(crate) fn rows_mut(pixmap: &mut Pixmap) -> std::slice::ChunksMut<'_, Px> {
    let width = to_usize(pixmap.width());
    let (pixels, _) = pixmap.data_mut().as_chunks_mut::<BYTES_PER_PX>();
    pixels.chunks_mut(width.max(1))
}

/// Strokes the segment `from`–`to` (pixels) `width_px` wide in `colour`, anti-aliased, with
/// round caps, so consecutive segments of a line join without gaps: a [`polyline`] of two
/// points.
pub(crate) fn stroke(
    pixmap: &mut Pixmap,
    from: [f64; 2],
    to: [f64; 2],
    width_px: f64,
    colour: Rgb,
) {
    let style = LineStyle {
        width_px,
        colour,
        dash: None,
    };
    polyline(pixmap, &[from, to], &style);
}

/// Fills `path` (pixels) in `colour`, anti-aliased, by the non-zero winding rule: shapes and
/// glyph outlines alike.
pub(crate) fn fill(pixmap: &mut Pixmap, path: &Path, colour: Rgb) {
    fill_alpha(pixmap, path, colour, u8::MAX);
}

/// Fills `path` (pixels) in `colour` with opacity `alpha` (255 is opaque), anti-aliased.
pub(crate) fn fill_alpha(pixmap: &mut Pixmap, path: &Path, colour: Rgb, alpha: u8) {
    let Some(path) = Bounds::of(pixmap).path(path) else {
        return;
    };
    pixmap.fill_path(
        &path,
        &paint_alpha(colour, alpha),
        FillRule::Winding,
        Transform::identity(),
        None,
    );
}

/// Strokes the outline of `path` (pixels) in `colour`, `width_px` wide, anti-aliased.
pub(crate) fn outline(pixmap: &mut Pixmap, path: &Path, width_px: f64, colour: Rgb) {
    let bounds = Bounds::of(pixmap);
    let Some(path) = bounds.path(path) else {
        return;
    };
    let stroke = Stroke {
        width: bounds.width(width_px),
        ..Stroke::default()
    };
    pixmap.stroke_path(&path, &paint(colour), &stroke, Transform::identity(), None);
}

/// Strokes a circle of `radius` (pixels) round `centre` (pixels) in `colour`, `width_px` wide,
/// anti-aliased; nothing for a radius that is no circle.
pub(crate) fn ring(pixmap: &mut Pixmap, centre: [f64; 2], radius: f64, width_px: f64, colour: Rgb) {
    let (x, y, r) = (to_f32(centre[0]), to_f32(centre[1]), to_f32(radius));
    if let Some(path) = PathBuilder::from_circle(x, y, r) {
        outline(pixmap, &path, width_px, colour);
    }
}

/// How a polyline is stroked.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct LineStyle {
    /// The width in pixels.
    pub width_px: f64,
    /// The colour.
    pub colour: Rgb,
    /// The dash pattern `[on, off]` in pixels; `None` is a solid line.
    pub dash: Option<[f32; 2]>,
}

/// Strokes the polyline through `points` (pixels) in `style`, with round caps and joins.
pub(crate) fn polyline(pixmap: &mut Pixmap, points: &[[f64; 2]], style: &LineStyle) {
    let Some((first, rest)) = points.split_first() else {
        return;
    };
    if rest.is_empty() {
        return;
    }
    let bounds = Bounds::of(pixmap);
    let mut path = PathBuilder::new();
    let (x, y) = bounds.point(*first);
    path.move_to(x, y);
    for p in rest {
        let (x, y) = bounds.point(*p);
        path.line_to(x, y);
    }
    let Some(path) = path.finish() else {
        return;
    };
    let stroke = Stroke {
        width: bounds.width(style.width_px),
        line_cap: LineCap::Round,
        line_join: LineJoin::Round,
        dash: style.dash.and_then(|d| StrokeDash::new(d.to_vec(), 0.0)),
        ..Stroke::default()
    };
    pixmap.stroke_path(
        &path,
        &paint(style.colour),
        &stroke,
        Transform::identity(),
        None,
    );
}

/// An opaque, anti-aliased paint of `colour`.
fn paint(colour: Rgb) -> Paint<'static> {
    paint_alpha(colour, u8::MAX)
}

/// An anti-aliased paint of `colour` with opacity `alpha`.
fn paint_alpha(colour: Rgb, alpha: u8) -> Paint<'static> {
    let mut paint = Paint::default();
    paint.set_color_rgba8(colour[0], colour[1], colour[2], alpha);
    paint.anti_alias = true;
    paint
}

/// `pixmap` as a PNG: 8-bit RGBA with straight alpha, `png`'s fdeflate compressor
/// (`Compression::Fast`) and the `Up` filter on every row, and no chunks beyond IHDR, IDAT and
/// IEND (no time, no text), so equal pixmaps give equal bytes.
///
/// # Errors
/// [`RenderError::Png`] when the encoder fails.
pub(crate) fn encode_png(pixmap: &Pixmap) -> Result<Vec<u8>, RenderError> {
    let mut straight = Vec::with_capacity(pixmap.data().len());
    for px in pixmap.pixels() {
        let c = px.demultiply();
        straight.extend_from_slice(&[c.red(), c.green(), c.blue(), c.alpha()]);
    }
    let mut out = Vec::new();
    let mut encoder = png::Encoder::new(&mut out, pixmap.width(), pixmap.height());
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    encoder.set_compression(png::Compression::Fast);
    // After `set_compression`, which picks the adaptive filter for `Fast`.
    encoder.set_filter(png::Filter::Up);
    let mut writer = encoder.write_header()?;
    writer.write_image_data(&straight)?;
    writer.finish()?;
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::test_support::decode;

    fn frame(rows: usize, cols: usize) -> MapFrame {
        MapFrame {
            rows,
            cols,
            cell_m: 50.0,
        }
    }

    #[test]
    fn the_pixmap_has_two_pixels_per_cell_or_fails_by_size() {
        let p = pixmap(&frame(3, 5)).unwrap();
        assert_eq!((p.width(), p.height()), (10, 6));
        assert!((metres_per_px(&frame(3, 5)) - 25.0).abs() < 1e-12);
        for (rows, cols) in [(0, 4), (4, 0), (usize::MAX, 1), (1, 1 << 40)] {
            let err = pixmap(&frame(rows, cols)).unwrap_err();
            assert!(
                matches!(err, RenderError::Size { rows: r, cols: c } if (r, c) == (rows, cols)),
                "{err}"
            );
        }
    }

    #[test]
    fn samples_blend_the_nearest_cell_centres_by_quarters() {
        // Cells 0, 4, 8 along a row: pixels sit at cell positions -0.25, 0.25, 0.75, ...
        let g = Grid::from_vec(1, 3, vec![0.0, 4.0, 8.0]).unwrap();
        let along: Vec<f64> = (0..6).map(|x| sample(&g, 0, x)).collect();
        assert_eq!(along, [0.0, 1.0, 3.0, 5.0, 7.0, 8.0]);
        assert_eq!(taps(0, 3), (0, 0));
        assert_eq!(taps(5, 3), (2, 2));
        assert_eq!(taps(9, 3), (2, 2));
        let col = Grid::from_vec(2, 1, vec![[0.0, 4.0, 8.0], [4.0, 4.0, 4.0]]).unwrap();
        assert_eq!(sample(&col, 1, 0), [1.0, 4.0, 7.0]);
        assert_eq!(sample(&col, 3, 1), [4.0; 3]);
    }

    #[test]
    fn rows_are_written_opaque_and_read_back_from_the_png() {
        let mut p = pixmap(&frame(1, 2)).unwrap();
        let rows: Vec<usize> = rows_mut(&mut p).map(|row| row.len()).collect();
        assert_eq!(rows, [4, 4]);
        for (y, row) in rows_mut(&mut p).enumerate() {
            for (x, px) in row.iter_mut().enumerate() {
                let v = u8::try_from(10 * x + 100 * y).unwrap();
                put(px, [v, v + 1, v + 2]);
            }
        }
        let png = encode_png(&p).unwrap();
        assert_eq!(&png[1..4], b"PNG");
        let image = decode(&png);
        assert_eq!((image.width, image.height), (4, 2));
        assert_eq!(image.rgba(3, 1), [130, 131, 132, 255]);
        assert_eq!(encode_png(&p).unwrap(), png);
    }

    #[test]
    fn a_stroke_covers_its_middle_in_its_colour() {
        let mut p = pixmap(&frame(8, 8)).unwrap();
        stroke(&mut p, [2.0, 8.0], [14.0, 8.0], 4.0, [10, 20, 30]);
        let image = decode(&encode_png(&p).unwrap());
        assert_eq!(image.rgba(8, 8), [10, 20, 30, 255]);
        assert_eq!(image.rgba(8, 2)[3], 0, "far from the line nothing is drawn");
        // A degenerate segment draws nothing and does not fail.
        stroke(&mut p, [f64::NAN, 0.0], [1.0, 1.0], 1.0, [0, 0, 0]);
    }

    // Final review round 4, finding 2: what reaches tiny-skia stays within the bounds of the
    // pixmap, so absurd but finite sizes are drawn clamped instead of failing inside it.
    #[test]
    fn the_bounds_leave_a_drawing_inside_them_alone_and_clamp_the_rest() {
        let p = pixmap(&frame(8, 4)).unwrap();
        let b = Bounds::of(&p);
        // 8 x 16 pixels: the box reaches 24 pixels beyond every side, a stroke 96 pixels wide.
        assert_eq!(
            b,
            Bounds {
                left: -24.0,
                top: -24.0,
                right: 32.0,
                bottom: 40.0,
                widest: 96.0
            }
        );
        assert_eq!(b.point([-0.0, 3.25]), (-0.0, 3.25));
        assert!(
            b.point([-0.0, 0.0]).0.is_sign_negative(),
            "inside, bit for bit"
        );
        assert_eq!(b.point([-1e300, 1e12]), (-24.0, 40.0));
        assert_eq!(b.point([f64::INFINITY, -50.0]), (32.0, -24.0));
        assert!(b.point([f64::NAN, 0.0]).0.is_nan());
        assert_eq!(
            (b.width(2.5), b.width(96.0), b.width(3e10)),
            (2.5, 96.0, 96.0)
        );
        let mut square = PathBuilder::new();
        square.move_to(1.0, 1.0);
        square.line_to(7.0, 1.0);
        square.quad_to(7.0, 4.0, 7.0, 7.0);
        square.cubic_to(5.0, 7.0, 3.0, 7.0, 1.0, 7.0);
        square.close();
        let square = square.finish().unwrap();
        assert!(matches!(b.path(&square), Some(Cow::Borrowed(_))));
        let huge = PathBuilder::from_circle(4.0, 8.0, 1e9).unwrap();
        let clamped = b.path(&huge).unwrap();
        let bounds = clamped.bounds();
        assert_eq!(
            (bounds.left(), bounds.top(), bounds.right(), bounds.bottom()),
            (-24.0, -24.0, 32.0, 40.0)
        );
    }

    #[test]
    fn absurd_sizes_are_drawn_clamped_without_failing() {
        let mut p = pixmap(&frame(8, 8)).unwrap();
        // A segment 3e10 pixels wide failed an assertion inside tiny-skia; clamped, it covers
        // every pixel, as the wider stroke would.
        stroke(&mut p, [2.0, 8.0], [14.0, 8.0], 3e10, [10, 20, 30]);
        let image = decode(&encode_png(&p).unwrap());
        assert_eq!(image.rgba(0, 0), [10, 20, 30, 255]);
        assert_eq!(image.rgba(15, 15), [10, 20, 30, 255]);
        let mut p = pixmap(&frame(8, 8)).unwrap();
        let huge = PathBuilder::from_circle(8.0, 8.0, 1e12).unwrap();
        fill(&mut p, &huge, [40, 50, 60]);
        outline(&mut p, &huge, 1e12, [70, 80, 90]);
        let far = PathBuilder::from_circle(1e12, -1e12, 3.0).unwrap();
        fill_alpha(&mut p, &far, [1, 2, 3], 128);
        let line = LineStyle {
            width_px: 2.0,
            colour: [5, 5, 5],
            dash: Some([3.0, 2.0]),
        };
        polyline(&mut p, &[[1.0, 1.0], [1e15, 1e15], [-1e300, 4.0]], &line);
        let image = decode(&encode_png(&p).unwrap());
        assert_eq!(image.width, 16);
    }
}
