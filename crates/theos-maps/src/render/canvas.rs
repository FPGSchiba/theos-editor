//! The frame-exact pixel grid of the renders and their PNG encoding.
//!
//! A frame of `rows` x `cols` cells is `2·rows` x `2·cols` square pixels; the pixel `(x, y)`
//! covers metres `[x, x + 1) · cell_m / 2` east and `[y, y + 1) · cell_m / 2` south (G19), so a
//! point in metres lands on the pixel `floor(m / (cell_m / 2))`. Cell rasters are sampled at
//! pixel centres by bilinear interpolation between cell centres, which at two pixels per cell
//! always takes the weights 3/4 and 1/4: exact in binary floating point.

use tiny_skia::{
    FillRule, LineCap, LineJoin, Paint, Path, PathBuilder, Pixmap, Stroke, StrokeDash, Transform,
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
/// round caps, so consecutive segments of a line join without gaps.
pub(crate) fn stroke(
    pixmap: &mut Pixmap,
    from: [f64; 2],
    to: [f64; 2],
    width_px: f64,
    colour: Rgb,
) {
    let mut path = PathBuilder::new();
    path.move_to(to_f32(from[0]), to_f32(from[1]));
    path.line_to(to_f32(to[0]), to_f32(to[1]));
    let Some(path) = path.finish() else {
        return;
    };
    let stroke = Stroke {
        width: to_f32(width_px),
        line_cap: LineCap::Round,
        ..Stroke::default()
    };
    pixmap.stroke_path(&path, &paint(colour), &stroke, Transform::identity(), None);
}

/// Fills `path` (pixels) in `colour`, anti-aliased, by the non-zero winding rule: shapes and
/// glyph outlines alike.
pub(crate) fn fill(pixmap: &mut Pixmap, path: &Path, colour: Rgb) {
    pixmap.fill_path(
        path,
        &paint(colour),
        FillRule::Winding,
        Transform::identity(),
        None,
    );
}

/// Fills `path` (pixels) in `colour` with opacity `alpha` (255 is opaque), anti-aliased.
pub(crate) fn fill_alpha(pixmap: &mut Pixmap, path: &Path, colour: Rgb, alpha: u8) {
    pixmap.fill_path(
        path,
        &paint_alpha(colour, alpha),
        FillRule::Winding,
        Transform::identity(),
        None,
    );
}

/// Strokes the outline of `path` (pixels) in `colour`, `width_px` wide, anti-aliased.
pub(crate) fn outline(pixmap: &mut Pixmap, path: &Path, width_px: f64, colour: Rgb) {
    let stroke = Stroke {
        width: to_f32(width_px),
        ..Stroke::default()
    };
    pixmap.stroke_path(path, &paint(colour), &stroke, Transform::identity(), None);
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
    let mut path = PathBuilder::new();
    path.move_to(to_f32(first[0]), to_f32(first[1]));
    for p in rest {
        path.line_to(to_f32(p[0]), to_f32(p[1]));
    }
    let Some(path) = path.finish() else {
        return;
    };
    let stroke = Stroke {
        width: to_f32(style.width_px),
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
}
