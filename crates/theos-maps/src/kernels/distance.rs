//! Exact Euclidean distance transform with nearest-cell indices.
//!
//! Felzenszwalb & Huttenlocher, "Distance Transforms of Sampled Functions", Theory of Computing
//! 8 (2012): the lower envelope of parabolas, applied to the columns and then to the rows of the
//! squared distances in cells.
//!
//! Ties between equally near cells go to the smallest flat index (row-major). The envelope keeps
//! every parabola that touches the minimum, also at a single point, and at a query that sits
//! exactly on a boundary it takes the smallest index of the parabolas that meet there. The squared
//! distances are whole numbers, so every comparison is exact.

use crate::grid::Grid;

use super::KernelError;
use super::num::to_f64;

/// "No site on this line".
const NONE: usize = usize::MAX;

/// The lower envelope of the parabolas rooted at the finite entries of `sq`: the positions of the
/// parabolas on it and where each starts to win (the first starts at minus infinity).
fn build(sq: &[f64]) -> (Vec<usize>, Vec<f64>) {
    let mut sites: Vec<usize> = Vec::new();
    let mut starts: Vec<f64> = Vec::new();
    for (pos, &value) in sq.iter().enumerate() {
        if !value.is_finite() {
            continue;
        }
        let at = to_f64(pos);
        loop {
            let Some(&last) = sites.last() else {
                sites.push(pos);
                starts.push(f64::NEG_INFINITY);
                break;
            };
            let last_at = to_f64(last);
            let cross =
                ((value + at * at) - (sq[last] + last_at * last_at)) / (2.0 * at - 2.0 * last_at);
            // Strict: a parabola that only touches the minimum at one point stays.
            if starts.last().is_some_and(|&start| cross < start) {
                sites.pop();
                starts.pop();
                continue;
            }
            sites.push(pos);
            starts.push(cross);
            break;
        }
    }
    (sites, starts)
}

/// One line of the transform: `sq[pos]` is the squared distance carried by position `pos`
/// (infinite for none) and `payload[pos]` the flat index of its nearest cell. Writes the lower
/// envelope's value and payload at every position into `out_sq` and `out_payload`.
fn envelope(sq: &[f64], payload: &[usize], out_sq: &mut [f64], out_payload: &mut [usize]) {
    let (sites, starts) = build(sq);
    let mut k = 0;
    for pos in 0..sq.len() {
        let at = to_f64(pos);
        while k + 1 < sites.len() && starts[k + 1] < at {
            k += 1;
        }
        let mut best = k;
        let mut end = k;
        while end + 1 < sites.len() && starts[end + 1].total_cmp(&at).is_eq() {
            end += 1;
            if payload[sites[end]] < payload[sites[best]] {
                best = end;
            }
        }
        if let Some(&site) = sites.get(best) {
            let gap = at - to_f64(site);
            out_sq[pos] = gap * gap + sq[site];
            out_payload[pos] = payload[site];
        } else {
            out_sq[pos] = f64::INFINITY;
            out_payload[pos] = NONE;
        }
    }
}

/// The exact Euclidean distance in metres from every cell to the nearest `true` cell, and the flat
/// index of that cell. Ties go to the smallest index.
///
/// # Errors
/// [`KernelError::EmptyMask`] when no cell is `true`.
pub fn distance(mask: &Grid<bool>, cell_m: f64) -> Result<(Grid<f64>, Grid<u32>), KernelError> {
    if !mask.as_slice().iter().any(|&b| b) {
        return Err(KernelError::EmptyMask);
    }
    let (rows, cols) = (mask.rows(), mask.cols());
    let mut d2 = Grid::filled(rows, cols, f64::INFINITY);
    let mut near = Grid::filled(rows, cols, NONE);
    // Columns: the squared distance in rows to the nearest `true` cell of the column.
    for c in 0..cols {
        let f: Vec<f64> = (0..rows)
            .map(|r| if mask.get(r, c) { 0.0 } else { f64::INFINITY })
            .collect();
        let payload: Vec<usize> = (0..rows).map(|r| mask.index(r, c)).collect();
        let mut out_f = vec![0.0; rows];
        let mut out_p = vec![NONE; rows];
        envelope(&f, &payload, &mut out_f, &mut out_p);
        for r in 0..rows {
            d2.set(r, c, out_f[r]);
            near.set(r, c, out_p[r]);
        }
    }
    // Rows: combine the columns' distances.
    for r in 0..rows {
        let span = r * cols..(r + 1) * cols;
        let f = d2.as_slice()[span.clone()].to_vec();
        let payload = near.as_slice()[span.clone()].to_vec();
        envelope(
            &f,
            &payload,
            &mut d2.as_mut_slice()[span.clone()],
            &mut near.as_mut_slice()[span],
        );
    }
    let dist = d2.map(|v| v.sqrt() * cell_m);
    let index = near.map(|i| u32::try_from(i).unwrap_or(u32::MAX));
    Ok((dist, index))
}

#[cfg(test)]
mod tests {
    use super::super::KernelError;
    use super::super::num::to_f64;
    use super::super::test_data::*;
    use super::*;
    use crate::grid::Grid;
    use crate::rng::StageRng;

    fn mask_from(rows: usize, cols: usize, v: &[u8]) -> Grid<bool> {
        Grid::from_vec(rows, cols, v.iter().map(|&b| b == 1).collect()).unwrap()
    }

    #[test]
    fn distance_matches_brute_force() {
        let mut rng = StageRng::for_key(1, "distance_matches_brute_force");
        let data: Vec<bool> = (0..20 * 17).map(|_| rng.uniform() < 0.07).collect();
        let mask = Grid::from_vec(20, 17, data).unwrap();
        let (dist, near) = distance(&mask, 1.5).unwrap();
        for i in 0..mask.len() {
            let (r, c) = mask.cell(i);
            let mut best: Option<(f64, usize)> = None;
            for j in 0..mask.len() {
                if !mask.as_slice()[j] {
                    continue;
                }
                let (rj, cj) = mask.cell(j);
                let dr = to_f64(r.abs_diff(rj));
                let dc = to_f64(c.abs_diff(cj));
                let d2 = dr * dr + dc * dc;
                if best.is_none_or(|(b, _)| d2 < b) {
                    best = Some((d2, j));
                }
            }
            let (d2, j) = best.unwrap();
            assert!(
                (dist.as_slice()[i] - d2.sqrt() * 1.5).abs() < 1e-12,
                "cell {i}"
            );
            assert_eq!(near.as_slice()[i] as usize, j, "cell {i}");
        }
    }

    #[test]
    fn distance_matches_scipy() {
        let mask = mask_from(6, 7, &SPARSE);
        let (dist, _) = distance(&mask, 2.0).unwrap();
        for (a, b) in dist.as_slice().iter().zip(EDT_SPARSE) {
            assert!((a - b).abs() < 1e-12);
        }
    }

    #[test]
    fn distance_of_an_empty_mask_is_an_error() {
        let mask = Grid::filled(4, 4, false);
        assert_eq!(distance(&mask, 1.0), Err(KernelError::EmptyMask));
        let none: Grid<bool> = Grid::filled(0, 0, false);
        assert_eq!(distance(&none, 1.0), Err(KernelError::EmptyMask));
    }

    #[test]
    fn ties_go_to_the_smallest_index() {
        // sites at (0, 0) and (0, 2): cell (0, 1) is equidistant, so index 0 wins
        let mut mask = Grid::filled(1, 3, false);
        mask.set(0, 0, true);
        mask.set(0, 2, true);
        let (_, near) = distance(&mask, 1.0).unwrap();
        assert_eq!(near.as_slice(), &[0, 0, 2]);
    }
}
