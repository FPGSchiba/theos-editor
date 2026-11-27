//! Positions and small shapes the interior renders share: raster cells to pixels, room centres
//! from the lattice, arrows and dots.

use tiny_skia::{PathBuilder, Pixmap};

use crate::kernels::num::to_f32;
use crate::render::canvas::{self, LineStyle, PX_PER_CELL};
use crate::render::fonts::Face;
use crate::render::palette::Rgb;
use crate::render::{RenderError, text};
use crate::rooms::RoomGraph;
use crate::rooms::lattice::Lattice;

/// A position in raster cells, `[row, col]`, fractions allowed.
pub(super) type Cell2 = [f64; 2];

/// Where the map's corner lies on the canvas.
#[derive(Debug, Clone, Copy)]
pub(super) struct Placement {
    /// The pixel of the map's north-west corner.
    pub offset: [f64; 2],
}

impl Placement {
    /// The pixel of the raster position `at`.
    pub(super) fn px(&self, at: Cell2) -> [f64; 2] {
        let per = f64::from(u32::try_from(PX_PER_CELL).unwrap_or(2));
        [self.offset[0] + at[1] * per, self.offset[1] + at[0] * per]
    }
}

/// The centre of every room, by room number: the mean of the block centres of its lattice
/// cells; `None` for a room without cells.
pub(super) fn centres(graph: &RoomGraph, lattice: &Lattice) -> Vec<Option<Cell2>> {
    graph
        .rooms
        .iter()
        .map(|room| {
            let mut sum = [0.0, 0.0];
            for cell in &room.cells {
                let b = lattice.block(*cell);
                sum[0] += f64::from(b.r0) + f64::from(b.r1);
                sum[1] += f64::from(b.c0) + f64::from(b.c1);
            }
            let n = f64::from(u32::try_from(room.cells.len()).unwrap_or(u32::MAX));
            (n > 0.0).then(|| [sum[0] / (2.0 * n), sum[1] / (2.0 * n)])
        })
        .collect()
}

/// A filled circle.
pub(super) fn dot(pixmap: &mut Pixmap, at: [f64; 2], radius: f64, colour: Rgb) {
    if let Some(path) = PathBuilder::from_circle(to_f32(at[0]), to_f32(at[1]), to_f32(radius)) {
        canvas::fill(pixmap, &path, colour);
    }
}

/// A line from `from` to `to` with an arrow head at `to`.
pub(super) fn arrow(pixmap: &mut Pixmap, from: [f64; 2], to: [f64; 2], width: f64, colour: Rgb) {
    let d = [to[0] - from[0], to[1] - from[1]];
    let len = libm::hypot(d[0], d[1]);
    if len < 1e-9 {
        return;
    }
    let u = [d[0] / len, d[1] / len];
    let head = (width * 2.5 + 3.0).min(len);
    let base = [to[0] - u[0] * head, to[1] - u[1] * head];
    let side = [-u[1] * head * 0.5, u[0] * head * 0.5];
    let line = LineStyle {
        width_px: width,
        colour,
        dash: None,
    };
    canvas::polyline(pixmap, &[from, base], &line);
    let mut b = PathBuilder::new();
    b.move_to(to_f32(to[0]), to_f32(to[1]));
    b.line_to(to_f32(base[0] + side[0]), to_f32(base[1] + side[1]));
    b.line_to(to_f32(base[0] - side[0]), to_f32(base[1] - side[1]));
    b.close();
    if let Some(path) = b.finish() {
        canvas::fill(pixmap, &path, colour);
    }
}

/// `label` centred on `at`, its middle on the pixel row of `at`.
///
/// # Errors
/// A font error from the glyph outlines.
pub(super) fn centred_text(
    pixmap: &mut Pixmap,
    face: &Face,
    label: &str,
    size: f32,
    at: [f64; 2],
    colour: Rgb,
) -> Result<(), RenderError> {
    let width = text::width(face, label, size);
    let origin = [to_f32(at[0]) - width / 2.0, to_f32(at[1]) + size * 0.35];
    text::draw(pixmap, face, label, size, origin, colour)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_raster_position_maps_two_pixels_per_cell_from_the_offset() {
        let at = Placement {
            offset: [3.0, 30.0],
        };
        assert_eq!(at.px([2.0, 5.0]), [13.0, 34.0]);
    }
}
