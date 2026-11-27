//! The buildable outlines of `envelope` (spec §5.5, §5.7) as pure functions on the frame.
//!
//! Circles are tested in integers on doubled coordinates: cell `(r, c)` has its centre at
//! `(2r + 1, 2c + 1)` and the frame's centre lies at `(rows, cols)`, so no square root is taken.

use crate::grid::Grid;
use crate::kernels::labels::{Connectivity, labels};

/// The width of the rim of a `ragged_circle` that the noise may erode, in raster cells
/// (0.3.0's `envelope` erosion reaches four cells in).
pub const RAGGED_RIM: i64 = 4;

/// The frame minus a margin of `margin` cells on every side.
#[must_use]
pub fn rect(rows: usize, cols: usize, margin: usize) -> Grid<bool> {
    let mut mask = Grid::filled(rows, cols, false);
    for r in margin..rows.saturating_sub(margin) {
        for c in margin..cols.saturating_sub(margin) {
            mask.set(r, c, true);
        }
    }
    mask
}

/// A cell's squared distance from the frame's centre and the squared doubled radius of the
/// largest centred circle minus `margin`, both in doubled coordinates.
struct Circle {
    rows: i64,
    cols: i64,
    /// The doubled radius: the shorter side minus twice the margin.
    diameter: i64,
}

impl Circle {
    fn new(rows: usize, cols: usize, margin: usize) -> Self {
        let wide = |v: usize| i64::try_from(v).unwrap_or(i64::MAX / 4);
        Self {
            rows: wide(rows),
            cols: wide(cols),
            diameter: (wide(rows.min(cols)) - 2 * wide(margin)).max(0),
        }
    }

    /// The squared distance of the centre of `(r, c)` from the frame's centre, doubled.
    fn d2(&self, r: usize, c: usize) -> i64 {
        let wide = |v: usize| i64::try_from(v).unwrap_or(i64::MAX / 4);
        let (y, x) = (2 * wide(r) + 1 - self.rows, 2 * wide(c) + 1 - self.cols);
        y * y + x * x
    }

    fn inside(&self, r: usize, c: usize) -> bool {
        self.d2(r, c) < self.diameter * self.diameter
    }

    /// Whether `(r, c)` lies within [`RAGGED_RIM`] cells of the circle's edge.
    fn in_rim(&self, r: usize, c: usize) -> bool {
        let inner = (self.diameter - 2 * RAGGED_RIM).max(0);
        self.d2(r, c) >= inner * inner
    }
}

/// The largest circle centred on the frame, minus a margin of `margin` cells: the cells whose
/// centre lies strictly inside it.
#[must_use]
pub fn circle(rows: usize, cols: usize, margin: usize) -> Grid<bool> {
    let shape = Circle::new(rows, cols, margin);
    let mut mask = Grid::filled(rows, cols, false);
    for i in 0..mask.len() {
        let (r, c) = mask.cell(i);
        mask.as_mut_slice()[i] = shape.inside(r, c);
    }
    mask
}

/// [`circle`] eroded where `noise` exceeds `t` within [`RAGGED_RIM`] cells of its edge, keeping
/// the 4-connected piece that holds the frame's centre cell `(rows / 2, cols / 2)`, which is
/// never eroded. `noise` has the frame's shape.
#[must_use]
pub fn ragged_circle(
    rows: usize,
    cols: usize,
    margin: usize,
    noise: &Grid<f64>,
    t: f64,
) -> Grid<bool> {
    let shape = Circle::new(rows, cols, margin);
    let centre = (rows / 2, cols / 2);
    let mut mask = circle(rows, cols, margin);
    for i in 0..mask.len() {
        let (r, c) = mask.cell(i);
        let eroded = (r, c) != centre
            && shape.in_rim(r, c)
            && noise.as_slice().get(i).is_some_and(|&n| n > t);
        if eroded {
            mask.as_mut_slice()[i] = false;
        }
    }
    let (pieces, _) = labels(&mask, Connectivity::Four);
    let home = if rows > 0 && cols > 0 {
        pieces.get(centre.0, centre.1)
    } else {
        0
    };
    for (m, &piece) in mask.as_mut_slice().iter_mut().zip(pieces.as_slice()) {
        *m = *m && home > 0 && piece == home;
    }
    mask
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kernels::noise::spectral_noise;
    use crate::rng::StageRng;

    fn count(mask: &Grid<bool>) -> usize {
        mask.as_slice().iter().filter(|&&m| m).count()
    }

    // AC: CE-80.3
    #[test]
    fn rect_is_the_frame_minus_the_margin() {
        let mask = rect(10, 12, 2);
        assert_eq!(count(&mask), 6 * 8);
        assert!(!mask.get(1, 5) && mask.get(2, 2) && mask.get(7, 9) && !mask.get(8, 9));
        assert_eq!(count(&rect(10, 12, 0)), 120);
        assert_eq!(count(&rect(4, 4, 3)), 0);
    }

    // AC: CE-80.1
    #[test]
    fn circle_is_centred_symmetric_and_inside_the_margin() {
        let mask = circle(20, 20, 1);
        for r in 0..20 {
            for c in 0..20 {
                assert_eq!(mask.get(r, c), mask.get(19 - r, c));
                assert_eq!(mask.get(r, c), mask.get(c, r));
            }
            assert!(!mask.get(0, r) && !mask.get(r, 0), "the margin stays free");
        }
        assert!(mask.get(10, 10) && mask.get(1, 9) && !mask.get(1, 4));
        let full = count(&circle(40, 40, 0));
        // pi r^2 with r = 20 is about 1257.
        assert!((1200..1300).contains(&full), "{full}");
    }

    // AC: CE-80.1
    #[test]
    fn ragged_circle_erodes_only_its_rim_and_stays_one_piece() {
        let (n, margin) = (48, 2);
        let whole = circle(n, n, margin);
        let shape = Circle::new(n, n, margin);
        for seed in 1..=4 {
            let noise = spectral_noise(
                n,
                n,
                &mut StageRng::for_key(seed, "outline"),
                2.4,
                1.0,
                None,
            );
            let mask = ragged_circle(n, n, margin, &noise, 0.0);
            assert!(
                count(&mask) < count(&whole),
                "seed {seed}: something eroded"
            );
            for i in 0..mask.len() {
                let (r, c) = mask.cell(i);
                assert!(
                    !mask.get(r, c) || whole.get(r, c),
                    "only cells of the circle"
                );
                if whole.get(r, c) && !shape.in_rim(r, c) {
                    // The disc inside the rim is never eroded and holds the centre.
                    assert!(mask.get(r, c), "seed {seed}: ({r}, {c}) off the rim");
                }
            }
            assert!(mask.get(n / 2, n / 2));
            assert_eq!(labels(&mask, Connectivity::Four).1, 1, "seed {seed}");
        }
        // A threshold above every noise value erodes nothing.
        let noise = Grid::filled(n, n, 0.0);
        assert_eq!(ragged_circle(n, n, margin, &noise, 1.0), whole);
    }
}
