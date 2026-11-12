//! Binary dilation, erosion and opening with scipy's `border_value = 0`: cells outside the grid
//! count as `false`.

use crate::grid::Grid;

use super::num::{to_f64, to_index};

/// A structuring element.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Structure {
    /// The cell and its four edge neighbours.
    Cross,
    /// Every offset `(dr, dc)` with `dr^2 + dc^2 <= (radius_m / cell_m)^2`. A negative or NaN
    /// radius leaves the centre only, and so does a ratio that is infinite or too large for an
    /// `isize` (a zero `cell_m`, an infinite `radius_m`); callers pass finite radii from validated
    /// recipes.
    Disk {
        /// Radius in metres.
        radius_m: f64,
    },
}

impl Structure {
    /// The offsets `(d_row, d_col)` of the element on a grid of `cell_m` metres.
    fn offsets(self, cell_m: f64) -> Vec<(isize, isize)> {
        let radius = match self {
            Self::Cross => return vec![(0, 0), (-1, 0), (0, -1), (0, 1), (1, 0)],
            Self::Disk { radius_m } => radius_m / cell_m,
        };
        if radius.is_nan() || radius < 0.0 {
            return vec![(0, 0)];
        }
        let limit = radius * radius;
        let reach = isize::try_from(to_index(radius, usize::MAX)).unwrap_or(0);
        let mut out = Vec::new();
        for dr in -reach..=reach {
            for dc in -reach..=reach {
                let (a, b) = (to_f64(dr.unsigned_abs()), to_f64(dc.unsigned_abs()));
                if a * a + b * b <= limit {
                    out.push((dr, dc));
                }
            }
        }
        out
    }
}

/// One pass: a cell is set when any (dilation) or all (erosion) of the cells at the offsets are
/// set; cells off the grid are `false`.
fn pass(mask: &Grid<bool>, offsets: &[(isize, isize)], dilate: bool) -> Grid<bool> {
    let mut out = Grid::filled(mask.rows(), mask.cols(), false);
    for i in 0..mask.len() {
        let hit = |d: &(isize, isize)| {
            let d = if dilate { (-d.0, -d.1) } else { *d };
            mask.neighbour(i, d).is_some_and(|j| mask.as_slice()[j])
        };
        out.as_mut_slice()[i] = if dilate {
            offsets.iter().any(hit)
        } else {
            offsets.iter().all(hit)
        };
    }
    out
}

/// scipy `binary_dilation(mask, structure, iterations, border_value=0)`. Zero iterations return
/// the mask (scipy would repeat until nothing changes).
#[must_use]
pub fn dilate(mask: &Grid<bool>, s: Structure, cell_m: f64, iterations: usize) -> Grid<bool> {
    let offsets = s.offsets(cell_m);
    (0..iterations).fold(mask.clone(), |g, _| pass(&g, &offsets, true))
}

/// scipy `binary_erosion(mask, structure, iterations, border_value=0)`. Zero iterations return
/// the mask.
#[must_use]
pub fn erode(mask: &Grid<bool>, s: Structure, cell_m: f64, iterations: usize) -> Grid<bool> {
    let offsets = s.offsets(cell_m);
    (0..iterations).fold(mask.clone(), |g, _| pass(&g, &offsets, false))
}

/// Opening: one erosion, then one dilation.
#[must_use]
pub fn open(mask: &Grid<bool>, s: Structure, cell_m: f64) -> Grid<bool> {
    dilate(&erode(mask, s, cell_m, 1), s, cell_m, 1)
}

#[cfg(test)]
mod tests {
    use super::super::test_data::*;
    use super::*;
    use crate::grid::Grid;

    fn blob() -> Grid<bool> {
        Grid::from_vec(6, 7, BLOB.iter().map(|&b| b == 1).collect()).unwrap()
    }

    fn bits(g: &Grid<bool>) -> Vec<u8> {
        g.as_slice().iter().map(|&b| u8::from(b)).collect()
    }

    // a 1.5-cell disk on 2 m cells
    const DISK: Structure = Structure::Disk { radius_m: 3.0 };

    #[test]
    fn dilation_and_erosion_match_scipy() {
        let g = blob();
        assert_eq!(bits(&dilate(&g, Structure::Cross, 2.0, 1)), BLOB_DIL_CROSS);
        assert_eq!(bits(&erode(&g, Structure::Cross, 2.0, 1)), BLOB_ERO_CROSS);
        assert_eq!(bits(&open(&g, Structure::Cross, 2.0)), BLOB_OPEN_CROSS);
        assert!(dilate(&g, DISK, 2.0, 1).as_slice().iter().all(|&b| b));
        assert_eq!(bits(&erode(&g, DISK, 2.0, 1)), BLOB_ERO_DISK);
        assert_eq!(bits(&open(&g, DISK, 2.0)), BLOB_OPEN_DISK);
        assert_eq!(bits(&erode(&g, Structure::Cross, 2.0, 2)), BLOB_ERO_CROSS_2);
    }

    #[test]
    fn a_two_cell_disk_has_the_right_shape() {
        let mut centre = Grid::filled(7, 7, false);
        centre.set(3, 3, true);
        let out = dilate(&centre, Structure::Disk { radius_m: 4.0 }, 2.0, 1);
        assert!(out.get(3 + 2, 3), "(2, 0) is in");
        assert!(out.get(3, 3 - 2), "(0, -2) is in");
        assert!(out.get(3 + 1, 3 + 1), "(1, 1) is in");
        assert!(!out.get(3 + 2, 3 + 1), "(2, 1) is out");
        assert!(!out.get(3 + 2, 3 + 2), "(2, 2) is out");
        assert_eq!(out.as_slice().iter().filter(|&&b| b).count(), 13);
    }

    #[test]
    fn erosion_clears_cells_at_the_border() {
        let full = Grid::filled(4, 5, true);
        let out = erode(&full, Structure::Cross, 1.0, 1);
        for r in 0..4 {
            for c in 0..5 {
                let inner = (1..3).contains(&r) && (1..4).contains(&c);
                assert_eq!(out.get(r, c), inner, "({r}, {c})");
            }
        }
    }

    #[test]
    fn zero_iterations_and_degenerate_disks_change_nothing() {
        let g = blob();
        assert_eq!(dilate(&g, Structure::Cross, 1.0, 0), g);
        assert_eq!(erode(&g, Structure::Cross, 1.0, 0), g);
        let tiny = Structure::Disk { radius_m: 0.4 };
        assert_eq!(dilate(&g, tiny, 1.0, 1), g);
        assert_eq!(erode(&g, Structure::Disk { radius_m: f64::NAN }, 1.0, 1), g);
    }
}
