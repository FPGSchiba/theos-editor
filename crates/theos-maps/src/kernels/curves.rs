//! Catmull-Rom curves and the signed distance to them, ported from 0.3.0 `common.py` with the
//! coordinate order `[x, y]`.

use super::KernelError;
use super::num::{sign, to_f64};
use super::voronoi::{PointIndex, same_shape};
use crate::grid::Grid;

/// The Catmull-Rom basis at `t` for one coordinate of four control points.
pub(super) fn spline(p: [f64; 4], t: f64) -> f64 {
    let [p0, p1, p2, p3] = p;
    0.5 * ((2.0 * p1)
        + (-p0 + p2) * t
        + (2.0 * p0 - 5.0 * p1 + 4.0 * p2 - p3) * (t * t)
        + (-p0 + 3.0 * p1 - 3.0 * p2 + p3) * (t * t * t))
}

/// A dense polyline through `points`: a straight line of `samples` points for two points,
/// otherwise a Catmull-Rom spline with phantom end points and `max(2, samples / (n - 1))`
/// samples per segment, ending exactly on the last point. Fewer than two points come back as
/// they are.
#[must_use]
pub fn dense_curve(points: &[[f64; 2]], samples: usize) -> Vec<[f64; 2]> {
    let n = points.len();
    if n < 2 {
        return points.to_vec();
    }
    if n == 2 {
        let last = to_f64(samples.saturating_sub(1));
        return (0..samples)
            .map(|k| {
                let t = if samples == 1 { 0.0 } else { to_f64(k) / last };
                let at = |c: usize| points[0][c] * (1.0 - t) + points[1][c] * t;
                [at(0), at(1)]
            })
            .collect();
    }
    let mut ext = Vec::with_capacity(n + 2);
    ext.push([
        points[0][0] * 2.0 - points[1][0],
        points[0][1] * 2.0 - points[1][1],
    ]);
    ext.extend_from_slice(points);
    ext.push([
        points[n - 1][0] * 2.0 - points[n - 2][0],
        points[n - 1][1] * 2.0 - points[n - 2][1],
    ]);
    let per = (samples / (n - 1)).max(2);
    let mut out = Vec::with_capacity(per * (n - 1) + 1);
    for quad in ext.windows(4) {
        for k in 0..per {
            let t = to_f64(k) / to_f64(per);
            let at = |c: usize| spline([quad[0][c], quad[1][c], quad[2][c], quad[3][c]], t);
            out.push([at(0), at(1)]);
        }
    }
    out.push(points[n - 1]);
    out
}

/// The signed and the unsigned distance of every grid point `(x, y)` to the polyline `curve`.
/// The nearest curve point comes from spade; the side is the sign of `t_x r_y - t_y r_x` with
/// the tangent `t` from the neighbours of that point (the point clamped to `1..len - 1`) and
/// `r` from it to the grid point. Returns `(signed, unsigned)`.
///
/// # Errors
/// [`KernelError::Empty`] for an empty curve, [`KernelError::Shape`] when the grids differ in
/// shape, [`KernelError::InvalidPoint`] for a curve point or grid cell that is not finite,
/// [`KernelError::Cancelled`] when `cancel` returns `true` (called every 4,096 curve points
/// and every 4,096 cells).
pub fn signed_distance(
    curve: &[[f64; 2]],
    x: &Grid<f64>,
    y: &Grid<f64>,
    cancel: &dyn Fn() -> bool,
) -> Result<(Grid<f64>, Grid<f64>), KernelError> {
    same_shape(x, y)?;
    let index = PointIndex::new(curve, cancel)?;
    let nearest_points = index.nearest_per_cell(x, y, cancel)?;
    let last = curve.len() - 1;
    let mut signed = Vec::with_capacity(x.len());
    let mut unsigned = Vec::with_capacity(x.len());
    let cells = x.as_slice().iter().zip(y.as_slice()).zip(nearest_points);
    for ((&qx, &qy), nearest) in cells {
        let (dx, dy) = (qx - curve[nearest][0], qy - curve[nearest][1]);
        let distance = (dx * dx + dy * dy).sqrt();
        let i = if last >= 2 {
            nearest.clamp(1, last - 1)
        } else {
            nearest
        };
        let (before, after) = (curve[i.saturating_sub(1)], curve[(i + 1).min(last)]);
        let (tx, ty) = (after[0] - before[0], after[1] - before[1]);
        let (rx, ry) = (qx - curve[i][0], qy - curve[i][1]);
        signed.push(distance * sign(tx * ry - ty * rx));
        unsigned.push(distance);
    }
    let build = |data: Vec<f64>| {
        Grid::from_vec(x.rows(), x.cols(), data).map_err(|_| KernelError::Shape {
            rows: x.rows(),
            cols: x.cols(),
            len: x.len(),
        })
    };
    Ok((build(signed)?, build(unsigned)?))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn grid(rows: usize, cols: usize, v: Vec<f64>) -> Grid<f64> {
        Grid::from_vec(rows, cols, v).unwrap()
    }

    #[test]
    fn dense_curve_passes_through_its_control_points() {
        let pts = [[0.0, 0.0], [1.0, 2.0], [3.0, 1.0], [4.0, 4.0]];
        let curve = dense_curve(&pts, 90);
        // 3 segments, 30 samples each, plus the final point.
        assert_eq!(curve.len(), 91);
        for (k, p) in pts.iter().enumerate() {
            let c = curve[k * 30];
            assert!((c[0] - p[0]).abs() < 1e-12 && (c[1] - p[1]).abs() < 1e-12);
        }
        let last = curve[curve.len() - 1];
        assert_eq!(last, pts[3]);
    }

    #[test]
    fn dense_curve_of_two_points_is_a_straight_line() {
        let curve = dense_curve(&[[0.0, 0.0], [10.0, 5.0]], 11);
        assert_eq!(curve.len(), 11);
        assert!((curve[5][0] - 5.0).abs() < 1e-12 && (curve[5][1] - 2.5).abs() < 1e-12);
        assert_eq!(curve[10], [10.0, 5.0]);
        assert_eq!(dense_curve(&[[1.0, 1.0], [2.0, 2.0]], 0).len(), 0);
        assert_eq!(dense_curve(&[[1.0, 1.0], [2.0, 2.0]], 1), vec![[1.0, 1.0]]);
        assert_eq!(dense_curve(&[[1.0, 1.0]], 5), vec![[1.0, 1.0]]);
    }

    #[test]
    fn signed_distance_flips_sign_across_a_straight_line() -> Result<(), KernelError> {
        let curve = dense_curve(&[[0.0, 0.0], [10.0, 0.0]], 50);
        let x = grid(1, 3, vec![5.0, 5.0, 5.0]);
        let y = grid(1, 3, vec![1.0, -1.0, 0.0]);
        let (signed, unsigned) = signed_distance(&curve, &x, &y, &|| false)?;
        assert!((signed.get(0, 0) - 1.0).abs() < 0.01);
        assert!((signed.get(0, 1) + 1.0).abs() < 0.01);
        assert!((unsigned.get(0, 0) - 1.0).abs() < 0.01);
        assert!((unsigned.get(0, 1) - 1.0).abs() < 0.01);
        assert!(unsigned.get(0, 2) < 0.11); // half the sample spacing, 10 / 49 / 2
        Ok(())
    }

    #[test]
    fn signed_distance_refuses_bad_input() {
        let one = grid(1, 1, vec![0.0]);
        let two = grid(1, 2, vec![0.0, 0.0]);
        let curve = [[0.0, 0.0], [1.0, 0.0], [2.0, 0.0]];
        assert_eq!(
            signed_distance(&[], &one, &one, &|| false),
            Err(KernelError::Empty)
        );
        assert!(matches!(
            signed_distance(&curve, &one, &two, &|| false),
            Err(KernelError::Shape { .. })
        ));
        let nan = grid(1, 1, vec![f64::NAN]);
        assert_eq!(
            signed_distance(&curve, &nan, &one, &|| false),
            Err(KernelError::InvalidPoint { index: 0 })
        );
        assert_eq!(
            signed_distance(&[[f64::NAN, 0.0]], &one, &one, &|| false),
            Err(KernelError::InvalidPoint { index: 0 })
        );
    }

    #[test]
    fn signed_distance_of_a_short_curve_uses_its_chord() -> Result<(), KernelError> {
        let x = grid(1, 1, vec![0.5]);
        let y = grid(1, 1, vec![2.0]);
        let (signed, _) = signed_distance(&[[0.0, 0.0], [1.0, 0.0]], &x, &y, &|| false)?;
        assert!(signed.get(0, 0) > 0.0);
        let (single, _) = signed_distance(&[[0.0, 0.0]], &x, &y, &|| false)?;
        assert!(single.get(0, 0).abs() < 1e-12);
        Ok(())
    }
}
