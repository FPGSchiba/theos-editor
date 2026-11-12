//! Lake outlines: marching squares on a mask, as closed rings in metres.
//!
//! The mask is padded with a `false` border, so every ring closes. Samples sit at cell centres and
//! the contour crosses each edge between an inside and an outside sample at its midpoint (the
//! level 0.5 of a binary field). A 2x2 quad of samples gives up to two directed segments with the
//! inside on the left; the diagonal saddles are cut as two separate corners (the 4-connected
//! reading). Every edge midpoint then has exactly one segment leaving it, so rings follow
//! unambiguously. Points are keyed by doubled integer lattice coordinates, which keeps the
//! tracing exact.

use std::collections::BTreeMap;

use super::num::to_f64;
use crate::grid::Grid;

/// A point on the doubled lattice of the padded mask: `(2 * row, 2 * col)` for a sample, with an
/// odd coordinate for the midpoint between two samples.
type Node = (usize, usize);

/// The segments of one quad, by its four inside flags (top-left, top-right, bottom-right,
/// bottom-left). `t`, `r`, `b`, `l` are the midpoints of the quad's four edges.
fn quad_segments(flags: [bool; 4], [t, r, b, l]: [Node; 4]) -> impl Iterator<Item = (Node, Node)> {
    let case = flags
        .iter()
        .enumerate()
        .fold(0_u8, |acc, (k, &on)| acc | (u8::from(on) << k));
    let list: [Option<(Node, Node)>; 2] = match case {
        1 => [Some((t, l)), None],
        2 => [Some((r, t)), None],
        3 => [Some((r, l)), None],
        4 => [Some((b, r)), None],
        5 => [Some((t, l)), Some((b, r))],
        6 => [Some((b, t)), None],
        7 => [Some((b, l)), None],
        8 => [Some((l, b)), None],
        9 => [Some((t, b)), None],
        10 => [Some((r, t)), Some((l, b))],
        11 => [Some((r, b)), None],
        12 => [Some((l, r)), None],
        13 => [Some((t, r)), None],
        14 => [Some((l, t)), None],
        _ => [None, None],
    };
    list.into_iter().flatten()
}

/// The outlines of the `true` regions of `mask` as closed rings of `[x, y]` points in metres,
/// the centre of cell `(r, c)` being at `((c + 0.5) * cell_m, (r + 0.5) * cell_m)`. A ring repeats
/// its first point at the end. Exterior rings have a positive shoelace area in `(x, y)`, holes a
/// negative one. Rings come in the scan order of their first segment.
#[must_use]
pub fn rings(mask: &Grid<bool>, cell_m: f64) -> Vec<Vec<[f64; 2]>> {
    // Sample (pr, pc) of the padded mask is mask cell (pr - 1, pc - 1).
    let inside = |pr: usize, pc: usize| {
        pr >= 1 && pc >= 1 && pr <= mask.rows() && pc <= mask.cols() && mask.get(pr - 1, pc - 1)
    };
    let mut order: Vec<Node> = Vec::new();
    let mut next: BTreeMap<Node, Node> = BTreeMap::new();
    for qr in 0..=mask.rows() {
        for qc in 0..=mask.cols() {
            let flags = [
                inside(qr, qc),
                inside(qr, qc + 1),
                inside(qr + 1, qc + 1),
                inside(qr + 1, qc),
            ];
            let mids = [
                (2 * qr, 2 * qc + 1),
                (2 * qr + 1, 2 * qc + 2),
                (2 * qr + 2, 2 * qc + 1),
                (2 * qr + 1, 2 * qc),
            ];
            for (from, to) in quad_segments(flags, mids) {
                order.push(from);
                next.insert(from, to);
            }
        }
    }
    let metres = |(row, col): Node| {
        [
            (to_f64(col) / 2.0 - 0.5) * cell_m,
            (to_f64(row) / 2.0 - 0.5) * cell_m,
        ]
    };
    let mut out = Vec::new();
    for start in order {
        if !next.contains_key(&start) {
            continue;
        }
        let mut ring = vec![metres(start)];
        let mut at = start;
        while let Some(to) = next.remove(&at) {
            ring.push(metres(to));
            at = to;
        }
        out.push(ring);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kernels::num::{max, min};

    fn area(ring: &[[f64; 2]]) -> f64 {
        ring.windows(2)
            .map(|w| w[0][0] * w[1][1] - w[1][0] * w[0][1])
            .sum::<f64>()
            / 2.0
    }

    fn block(rows: usize, cols: usize, on: impl Fn(usize, usize) -> bool) -> Grid<bool> {
        let mut g = Grid::filled(rows, cols, false);
        for r in 0..rows {
            for c in 0..cols {
                g.set(r, c, on(r, c));
            }
        }
        g
    }

    #[test]
    fn a_three_by_three_lake_ring_has_area_8_5_cells() {
        let mask = block(7, 7, |r, c| (2..5).contains(&r) && (2..5).contains(&c));
        let out = rings(&mask, 10.0);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].first(), out[0].last());
        assert!((area(&out[0]) - 8.5 * 100.0).abs() < 1e-9);
    }

    #[test]
    fn a_lake_with_an_island_gives_a_hole() {
        let mask = block(9, 9, |r, c| {
            (2..7).contains(&r) && (2..7).contains(&c) && !(r == 4 && c == 4)
        });
        let out = rings(&mask, 1.0);
        assert_eq!(out.len(), 2);
        let mut areas: Vec<f64> = out.iter().map(|r| area(r)).collect();
        areas.sort_by(f64::total_cmp);
        // the hole is a diamond with its corners half a cell from the island centre: area 0.5
        assert!((areas[0] + 0.5).abs() < 1e-9, "{areas:?}");
        assert!(areas[1] > 0.0);
    }

    #[test]
    fn rings_close_at_the_map_edge() {
        let mask = Grid::filled(3, 3, true);
        let out = rings(&mask, 2.0);
        assert_eq!(out.len(), 1);
        let ring = &out[0];
        assert_eq!(ring.first(), ring.last());
        let min_x = ring.iter().fold(f64::INFINITY, |a, p| min(a, p[0]));
        let max_x = ring.iter().fold(f64::NEG_INFINITY, |a, p| max(a, p[0]));
        assert!(min_x.abs() < 1e-12 && (max_x - 6.0).abs() < 1e-12);
        assert!((area(ring) - 8.5 * 4.0).abs() < 1e-9);
    }

    #[test]
    fn diagonal_cells_are_separate_rings_and_an_empty_mask_has_none() {
        let mask = block(4, 4, |r, c| r == c && r < 2);
        let out = rings(&mask, 1.0);
        assert_eq!(out.len(), 2);
        assert!(out.iter().all(|r| area(r) > 0.0));
        assert_eq!(rings(&Grid::filled(3, 3, false), 1.0).len(), 0);
    }
}
