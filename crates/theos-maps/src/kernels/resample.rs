//! Resampling of a parent layer into a child frame. A child cell takes the parent sampled at the
//! cell's centre: by Catmull-Rom for measured layers, by the nearest parent cell for class layers
//! (L11), so that an interpolated code such as 1.5 never stands for a class that does not exist.

use super::KernelError;
use super::cancel_point;
use super::curves::spline;
use super::num::{max, min, to_f64, to_index};
use crate::grid::Grid;

/// How a child cell reads its parent: [`Sampling::CatmullRom`] for measured layers (heights,
/// moisture), [`Sampling::Nearest`] for class layers (codes such as `water`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Sampling {
    /// Catmull-Rom over the 4 × 4 parent cells around the sample point.
    CatmullRom,
    /// The parent cell that contains the sample point.
    Nearest,
}

/// The child grid to sample: `rows × cols` cells of `cell_m` whose north-west corner lies at
/// `origin_m` in the parent's frame.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ChildGrid {
    /// The north-west corner of the child, in metres from the parent's north-west corner.
    pub origin_m: [f64; 2],
    /// Rows of child cells.
    pub rows: usize,
    /// Columns of child cells.
    pub cols: usize,
    /// Side of a child cell, in metres.
    pub cell_m: f64,
}

/// `parent` (rows × cols of `parent_cell_m`) sampled at the centres of the `child` grid. Every
/// sample point is clamped to the parent's grid, so cells past its edge read the nearest edge
/// cell. An empty child gives an empty grid.
///
/// # Errors
/// [`KernelError::BadGeometry`] when a cell size is not positive and finite or the origin is not
/// finite, [`KernelError::Shape`] when the child's cell count overflows `usize`,
/// [`KernelError::NoCells`] when the child has cells but the parent has none,
/// [`KernelError::Cancelled`] when `cancel` returns `true` (called every 4,096 child cells).
pub fn resample(
    parent: &Grid<f64>,
    parent_cell_m: f64,
    child: &ChildGrid,
    sampling: Sampling,
    cancel: &dyn Fn() -> bool,
) -> Result<Grid<f64>, KernelError> {
    let positive = |x: f64| x.is_finite() && x > 0.0;
    let origin_m = child.origin_m;
    if !(positive(parent_cell_m)
        && positive(child.cell_m)
        && origin_m.iter().all(|v| v.is_finite()))
    {
        return Err(KernelError::BadGeometry);
    }
    let (rows, cols) = (child.rows, child.cols);
    let len = rows.checked_mul(cols).ok_or(KernelError::Shape {
        rows,
        cols,
        len: usize::MAX,
    })?;
    if len > 0 && parent.is_empty() {
        return Err(KernelError::NoCells);
    }
    let mut data = Vec::with_capacity(len);
    for r in 0..rows {
        let ey = centre_in_parent(origin_m[1], child.cell_m, r, parent_cell_m);
        for c in 0..cols {
            cancel_point(data.len(), cancel)?;
            let ex = centre_in_parent(origin_m[0], child.cell_m, c, parent_cell_m);
            data.push(match sampling {
                Sampling::Nearest => nearest(parent, ex, ey),
                Sampling::CatmullRom => catmull_rom(parent, ex - 0.5, ey - 0.5),
            });
        }
    }
    Grid::from_vec(rows, cols, data).map_err(|_| KernelError::Shape { rows, cols, len })
}

/// The centre of child cell `k` along one axis, in parent cell units, measured from the
/// parent's north-west corner: a parent cell `i` spans `i..i + 1`.
fn centre_in_parent(origin: f64, cell_m: f64, k: usize, parent_cell_m: f64) -> f64 {
    (origin + (to_f64(k) + 0.5) * cell_m) / parent_cell_m
}

/// The parent cell that contains the point at `(ex, ey)` in parent cell units, clamped to the
/// grid.
fn nearest(parent: &Grid<f64>, ex: f64, ey: f64) -> f64 {
    parent.get(to_index(ey, parent.rows()), to_index(ex, parent.cols()))
}

/// Catmull-Rom at `(u, v)` in parent cell-centre units (cell `i` centred on `i`), with the
/// point clamped to the grid and every stencil index clamped to the grid.
fn catmull_rom(parent: &Grid<f64>, u: f64, v: f64) -> f64 {
    let (rows, cols) = (parent.rows(), parent.cols());
    let u = min(max(u, 0.0), to_f64(cols.saturating_sub(1)));
    let v = min(max(v, 0.0), to_f64(rows.saturating_sub(1)));
    let (ci, ri) = (to_index(u, cols), to_index(v, rows));
    let (tu, tv) = (u - to_f64(ci), v - to_f64(ri));
    let mut column = [0.0; 4];
    for (k, slot) in column.iter_mut().enumerate() {
        let r = stencil(ri, k, rows);
        let row = [0, 1, 2, 3].map(|j| parent.get(r, stencil(ci, j, cols)));
        *slot = spline(row, tu);
    }
    spline(column, tv)
}

/// The `k`-th of the four stencil indices around cell `i` (`i - 1 ..= i + 2`), clamped to
/// `0..len`.
fn stencil(i: usize, k: usize, len: usize) -> usize {
    (i + k).saturating_sub(1).min(len.saturating_sub(1))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn never() -> bool {
        false
    }

    /// A parent of `rows × cols` cells, `value(r, c)` in each cell.
    fn parent(rows: usize, cols: usize, value: impl Fn(usize, usize) -> f64) -> Grid<f64> {
        let data = (0..rows)
            .flat_map(|r| (0..cols).map(move |c| (r, c)))
            .map(|(r, c)| value(r, c))
            .collect();
        Grid::from_vec(rows, cols, data).unwrap_or_else(|_| Grid::filled(0, 0, 0.0))
    }

    /// A child grid of `rows × cols` cells of `cell_m` from `origin_m`.
    fn child(origin_m: [f64; 2], rows: usize, cols: usize, cell_m: f64) -> ChildGrid {
        ChildGrid {
            origin_m,
            rows,
            cols,
            cell_m,
        }
    }

    /// Centre of cell `k` along an axis that starts at `origin` with cells of `cell`.
    fn centre(origin: f64, cell: f64, k: usize) -> f64 {
        origin + (to_f64(k) + 0.5) * cell
    }

    #[test]
    fn resample_reproduces_a_linear_ramp_inside_the_grid() -> Result<(), KernelError> {
        // Parent: 8 m cells, h = 3x + 2y at the cell centres.
        let h = |r: usize, c: usize| 3.0 * centre(0.0, 8.0, c) + 2.0 * centre(0.0, 8.0, r);
        let p = parent(12, 12, h);
        // Child: 2 m cells over x, y in 13..71 m, all inside the parent's stencils.
        let out = resample(
            &p,
            8.0,
            &child([12.0, 12.0], 30, 30, 2.0),
            Sampling::CatmullRom,
            &never,
        )?;
        for r in 0..30 {
            for c in 0..30 {
                let want = 3.0 * centre(12.0, 2.0, c) + 2.0 * centre(12.0, 2.0, r);
                assert!(
                    (out.get(r, c) - want).abs() < 1e-9,
                    "cell ({r}, {c}): {} != {want}",
                    out.get(r, c)
                );
            }
        }
        Ok(())
    }

    #[test]
    fn resample_hits_parent_values_at_parent_centres() -> Result<(), KernelError> {
        // Power-of-two ratio (8 m parent, 2 m child): with origin -1 m, child cells 2 and 6 mod 4
        // sit exactly on parent centres (4 m, 12 m, ...), so the sample is exact in binary.
        let p = parent(4, 4, |r, c| to_f64(r * 4 + c) + 0.125);
        for sampling in [Sampling::CatmullRom, Sampling::Nearest] {
            let out = resample(&p, 8.0, &child([-1.0, -1.0], 16, 16, 2.0), sampling, &never)?;
            for r in (2..16).step_by(4) {
                for c in (2..16).step_by(4) {
                    let want = p.get(r / 4, c / 4);
                    assert_eq!(
                        out.get(r, c).to_bits(),
                        want.to_bits(),
                        "{sampling:?} cell ({r}, {c})"
                    );
                }
            }
        }
        Ok(())
    }

    #[test]
    fn resample_clamps_samples_past_the_parent_edge() -> Result<(), KernelError> {
        let p = parent(4, 4, |r, c| to_f64(r * 4 + c) + 1.0);
        let nearest = resample(
            &p,
            8.0,
            &child([-100.0, -100.0], 4, 4, 2.0),
            Sampling::Nearest,
            &never,
        )?;
        for &v in nearest.as_slice() {
            assert_eq!(v.to_bits(), p.get(0, 0).to_bits());
        }
        let smooth = resample(
            &p,
            8.0,
            &child([-100.0, -100.0], 4, 4, 2.0),
            Sampling::CatmullRom,
            &never,
        )?;
        for &v in smooth.as_slice() {
            assert!(v.is_finite());
            assert_eq!(v.to_bits(), p.get(0, 0).to_bits());
        }
        let far = resample(
            &p,
            8.0,
            &child([200.0, 200.0], 4, 4, 2.0),
            Sampling::Nearest,
            &never,
        )?;
        for &v in far.as_slice() {
            assert_eq!(v.to_bits(), p.get(3, 3).to_bits());
        }
        Ok(())
    }

    #[test]
    fn nearest_keeps_class_codes() -> Result<(), KernelError> {
        // Checkerboard of codes 0 and 3 on 8 m cells; the child starts west and north of the
        // parent, so some samples clamp to the edge.
        let p = parent(3, 4, |r, c| if (r + c) % 2 == 0 { 0.0 } else { 3.0 });
        let (origin, pc) = ([-3.0, -5.0], 8.0);
        let grid = child(origin, 9, 13, 2.0);
        let out = resample(&p, pc, &grid, Sampling::Nearest, &never)?;
        for r in 0..grid.rows {
            for c in 0..grid.cols {
                // The parent cell containing the child centre, clamped to the parent grid.
                let ex = libm::floor(centre(origin[0], grid.cell_m, c) / pc);
                let ey = libm::floor(centre(origin[1], grid.cell_m, r) / pc);
                let want = p.get(to_index(ey, p.rows()), to_index(ex, p.cols()));
                assert_eq!(
                    out.get(r, c).to_bits(),
                    want.to_bits(),
                    "cell ({r}, {c}): got {}, want {want}",
                    out.get(r, c)
                );
            }
        }
        Ok(())
    }

    #[test]
    fn resample_stops_at_a_cancel_point() {
        let p = parent(4, 4, |_, _| 1.0);
        let cancelled = resample(
            &p,
            8.0,
            &child([0.0, 0.0], 100, 100, 2.0),
            Sampling::Nearest,
            &|| true,
        );
        assert_eq!(cancelled, Err(KernelError::Cancelled));
    }

    #[test]
    fn resample_refuses_bad_geometry_and_empty_parents() {
        let p = parent(4, 4, |_, _| 1.0);
        let empty = parent(0, 0, |_, _| 0.0);
        assert_eq!(
            resample(
                &p,
                0.0,
                &child([0.0, 0.0], 2, 2, 2.0),
                Sampling::Nearest,
                &never
            ),
            Err(KernelError::BadGeometry)
        );
        assert_eq!(
            resample(
                &p,
                8.0,
                &child([f64::NAN, 0.0], 2, 2, 2.0),
                Sampling::Nearest,
                &never
            ),
            Err(KernelError::BadGeometry)
        );
        assert_eq!(
            resample(
                &empty,
                8.0,
                &child([0.0, 0.0], 2, 2, 2.0),
                Sampling::CatmullRom,
                &never
            ),
            Err(KernelError::NoCells)
        );
        let none = resample(
            &empty,
            8.0,
            &child([0.0, 0.0], 0, 0, 2.0),
            Sampling::CatmullRom,
            &never,
        );
        assert_eq!(none.map(|g| g.len()), Ok(0));
    }

    #[test]
    fn resample_refuses_a_child_whose_cell_count_overflows() {
        let p = parent(4, 4, |_, _| 1.0);
        assert_eq!(
            resample(
                &p,
                8.0,
                &child([0.0, 0.0], usize::MAX, 2, 2.0),
                Sampling::Nearest,
                &never
            ),
            Err(KernelError::Shape {
                rows: usize::MAX,
                cols: 2,
                len: usize::MAX,
            })
        );
    }
}
