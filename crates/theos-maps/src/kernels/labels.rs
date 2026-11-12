//! Connected-component labels with `scipy.ndimage.label` numbering.

use crate::grid::{D8, Grid};

/// Which cells touch: the four edge neighbours, or all eight.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Connectivity {
    /// North, east, south and west.
    Four,
    /// The four edge neighbours and the four diagonals.
    Eight,
}

/// Labels the connected components of `mask`: `0` is background and the components are numbered
/// `1..=n` by the row-major position of their first cell, like `scipy.ndimage.label`. Returns the
/// labels and `n`.
#[must_use]
pub fn labels(mask: &Grid<bool>, c: Connectivity) -> (Grid<u32>, u32) {
    let step = match c {
        Connectivity::Four => 2,
        Connectivity::Eight => 1,
    };
    let mut out = Grid::filled(mask.rows(), mask.cols(), 0_u32);
    let mut count = 0_u32;
    let mut stack: Vec<usize> = Vec::new();
    for start in 0..mask.len() {
        if !mask.as_slice()[start] || out.as_slice()[start] != 0 {
            continue;
        }
        count += 1;
        out.as_mut_slice()[start] = count;
        stack.push(start);
        while let Some(i) = stack.pop() {
            for &d in D8.iter().step_by(step) {
                if let Some(j) = mask.neighbour(i, d)
                    && mask.as_slice()[j]
                    && out.as_slice()[j] == 0
                {
                    out.as_mut_slice()[j] = count;
                    stack.push(j);
                }
            }
        }
    }
    (out, count)
}

#[cfg(test)]
mod tests {
    use super::super::test_data::*;
    use super::*;
    use crate::grid::Grid;

    fn sparse() -> Grid<bool> {
        Grid::from_vec(6, 7, SPARSE.iter().map(|&b| b == 1).collect()).unwrap()
    }

    #[test]
    fn labels_number_components_in_scan_order() {
        let (l, n) = labels(&sparse(), Connectivity::Four);
        assert_eq!(n, 7);
        assert_eq!(l.as_slice(), &LABELS_4);
    }

    #[test]
    fn eight_connectivity_joins_diagonals() {
        let (l, n) = labels(&sparse(), Connectivity::Eight);
        assert_eq!(n, 4);
        assert_eq!(l.as_slice(), &LABELS_8);
    }

    #[test]
    fn an_empty_mask_has_no_components() {
        let (l, n) = labels(&Grid::filled(3, 3, false), Connectivity::Four);
        assert_eq!(n, 0);
        assert!(l.as_slice().iter().all(|&v| v == 0));
    }
}
