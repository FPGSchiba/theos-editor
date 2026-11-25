//! Where the map lies on a canvas: the pixel of a point in metres.

use super::canvas;
use crate::world::MapFrame;

/// The map on a canvas: its top-left corner in pixels and the metres one pixel spans.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct View {
    /// The pixel of the map's north-west corner.
    pub offset: [f64; 2],
    /// Metres per pixel.
    pub metres_per_px: f64,
}

impl View {
    /// The view of `frame` with its corner at `offset` pixels.
    pub(crate) fn new(frame: &MapFrame, offset: [f64; 2]) -> Self {
        Self {
            offset,
            metres_per_px: canvas::metres_per_px(frame),
        }
    }

    /// The pixel of the point `at` (metres east and south of the corner).
    pub(crate) fn px(&self, at: [f64; 2]) -> [f64; 2] {
        [
            self.offset[0] + at[0] / self.metres_per_px,
            self.offset[1] + at[1] / self.metres_per_px,
        ]
    }

    /// `metres` as pixels.
    pub(crate) fn len_px(&self, metres: f64) -> f64 {
        metres / self.metres_per_px
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn points_map_to_pixels_with_the_offset() {
        let frame = MapFrame {
            rows: 4,
            cols: 4,
            cell_m: 50.0,
        };
        let view = View::new(&frame, [3.0, 10.0]);
        assert_eq!(view.px([50.0, 100.0]), [5.0, 14.0]);
        assert!((view.len_px(25.0) - 1.0).abs() < 1e-12);
    }
}
