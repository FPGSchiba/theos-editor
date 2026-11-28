//! Cell-edge outlines: the exact boundary of a mask along cell edges, as closed integer rings.
//!
//! Marching squares (`contour`) cuts corners; this kernel does not, so a polygon built from a
//! ring encloses exactly the cells of its region.
//!
//! Coordinates are `[col, row]` grid corners: cell `(r, c)` spans x from `c` to `c + 1` and y from
//! `r` to `r + 1`, with y growing south. Exterior rings have a positive shoelace sum
//! `sum(x_i * y_(i+1) - x_(i+1) * y_i)` in these coordinates (counter-clockwise with y up,
//! clockwise on screen); holes have a negative one. Callers that need the other handedness flip y.
//!
//! Rings are closed (the first vertex repeats at the end), start at their lowest-then-leftmost
//! corner and have no collinear vertices. All exterior rings come first, in the scan order of
//! their first corner (row, then column); the holes follow, in the same order. Two cells that only
//! touch at a corner belong to separate rings (4-connectivity).

use std::collections::BTreeMap;

use crate::grid::Grid;

/// A grid corner as `(row, col)`, so that the map order is the scan order.
type Corner = (i64, i64);

/// Directed boundary edges keyed by start corner. The interior is on the right of travel in
/// `(x east, y south)` terms, which makes exterior rings positive.
type Edges = BTreeMap<Corner, Vec<Corner>>;

/// Twice the signed area of a ring (shoelace sum) of `(row, col)` corners, open (first vertex not
/// repeated).
fn shoelace(ring: &[Corner]) -> i64 {
    ring.iter()
        .zip(ring.iter().cycle().skip(1))
        .map(|(&(r0, c0), &(r1, c1))| c0 * r1 - c1 * r0)
        .sum()
}

/// A grid coordinate as a signed corner coordinate; a grid never gets near `i64::MAX`.
fn signed(i: usize) -> i64 {
    i64::try_from(i).unwrap_or(i64::MAX)
}

/// The directed boundary edges of the `true` cells of `mask`.
fn boundary_edges(mask: &Grid<bool>) -> Edges {
    let off = |r: usize, c: usize, d: (isize, isize)| {
        mask.neighbour_at(r, c, d)
            .is_none_or(|i| !mask.as_slice()[i])
    };
    let mut edges = Edges::new();
    for r in 0..mask.rows() {
        for c in 0..mask.cols() {
            if !mask.get(r, c) {
                continue;
            }
            let (y, x) = (signed(r), signed(c));
            let mut add = |from: Corner, to: Corner| edges.entry(from).or_default().push(to);
            if off(r, c, (-1, 0)) {
                add((y, x), (y, x + 1));
            }
            if off(r, c, (0, 1)) {
                add((y, x + 1), (y + 1, x + 1));
            }
            if off(r, c, (1, 0)) {
                add((y + 1, x + 1), (y + 1, x));
            }
            if off(r, c, (0, -1)) {
                add((y + 1, x), (y, x));
            }
        }
    }
    edges
}

/// Rank of the turn from `from -> via` onto `via -> to`: 0 right, 1 straight, 2 left. Taking the
/// right turn (towards the interior) keeps cells that only touch at a corner in separate rings.
fn turn_rank(from: Corner, via: Corner, to: Corner) -> i64 {
    let (d, e) = (
        (via.0 - from.0, via.1 - from.1),
        (to.0 - via.0, to.1 - via.1),
    );
    match (d.1 * e.0 - d.0 * e.1).signum() {
        1 => 0,
        0 => 1,
        _ => 2,
    }
}

/// Drops the vertices that lie on a straight line between their neighbours.
fn drop_collinear(ring: &[Corner]) -> Vec<Corner> {
    let n = ring.len();
    (0..n)
        .filter(|&i| {
            let (p, q, s) = (ring[(i + n - 1) % n], ring[i], ring[(i + 1) % n]);
            (q.0 - p.0) * (s.1 - q.1) - (q.1 - p.1) * (s.0 - q.0) != 0
        })
        .map(|i| ring[i])
        .collect()
}

/// Removes the edge `from -> to` from `edges`.
fn take(edges: &mut Edges, from: Corner, to: Corner) {
    if let Some(ends) = edges.get_mut(&from) {
        if let Some(i) = ends.iter().position(|&e| e == to) {
            ends.remove(i);
        }
        if ends.is_empty() {
            edges.remove(&from);
        }
    }
}

/// Follows edges from the first one at the smallest corner until the ring closes; returns its
/// corners without the repeated first one.
fn trace(edges: &mut Edges) -> Vec<Corner> {
    let Some((&start, ends)) = edges.iter().next() else {
        return Vec::new();
    };
    let first = ends[0];
    take(edges, start, first);
    let mut ring = vec![start];
    let (mut from, mut via) = (start, first);
    while via != start {
        ring.push(via);
        let next = edges.get(&via).and_then(|ends| {
            ends.iter()
                .copied()
                .min_by_key(|&e| turn_rank(from, via, e))
        });
        let Some(next) = next else { break };
        take(edges, via, next);
        (from, via) = (via, next);
    }
    ring
}

/// The cell-edge boundary rings of the `true` region of `mask` as `[col, row]` corners: exterior
/// rings first, then holes (see the module docs for orientation and order).
#[must_use]
pub fn outline(mask: &Grid<bool>) -> Vec<Vec<[i64; 2]>> {
    let mut edges = boundary_edges(mask);
    let mut exteriors: Vec<Vec<[i64; 2]>> = Vec::new();
    let mut holes: Vec<Vec<[i64; 2]>> = Vec::new();
    while !edges.is_empty() {
        let mut ring = drop_collinear(&trace(&mut edges));
        let lowest = (0..ring.len()).min_by_key(|&i| ring[i]).unwrap_or(0);
        ring.rotate_left(lowest);
        let exterior = shoelace(&ring) > 0;
        let closed = ring
            .iter()
            .chain(ring.first())
            .map(|&(r, c)| [c, r])
            .collect();
        if exterior {
            exteriors.push(closed);
        } else {
            holes.push(closed);
        }
    }
    exteriors.append(&mut holes);
    exteriors
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::StageRng;

    fn mask(rows: &[&str]) -> Grid<bool> {
        let cols = rows[0].len();
        let data = rows
            .iter()
            .flat_map(|r| r.chars().map(|ch| ch == '#'))
            .collect();
        Grid::from_vec(rows.len(), cols, data).expect("shape")
    }

    fn area2(ring: &[[i64; 2]]) -> i64 {
        ring.windows(2)
            .map(|w| w[0][0] * w[1][1] - w[1][0] * w[0][1])
            .sum()
    }

    #[test]
    fn single_cell_square() {
        let rings = outline(&mask(&["#"]));
        assert_eq!(rings, vec![vec![[0, 0], [1, 0], [1, 1], [0, 1], [0, 0]]]);
        assert_eq!(area2(&rings[0]), 2);
    }

    #[test]
    fn l_shape_has_six_vertices() {
        let rings = outline(&mask(&["#.", "##"]));
        assert_eq!(rings.len(), 1);
        assert_eq!(rings[0].len(), 7);
        assert_eq!(rings[0][0], rings[0][6]);
        assert_eq!(area2(&rings[0]), 6);
    }

    #[test]
    fn ring_with_hole() {
        let rings = outline(&mask(&["###", "#.#", "###"]));
        assert_eq!(rings.len(), 2);
        assert_eq!(area2(&rings[0]), 18);
        assert_eq!(area2(&rings[1]), -2);
        assert_eq!(rings[1].len(), 5);
    }

    #[test]
    fn diagonal_touch_gives_two_rings() {
        let rings = outline(&mask(&["#.", ".#"]));
        assert_eq!(rings.len(), 2);
        assert!(rings.iter().all(|r| r.len() == 5 && area2(r) == 2));
    }

    #[test]
    fn area_equals_cell_count() {
        for seed in 1..=10 {
            let mut rng = StageRng::for_key(seed, "outline_area");
            let data: Vec<bool> = (0..12 * 15).map(|_| rng.below(100) < 55).collect();
            let count = signed(data.iter().filter(|&&b| b).count());
            let grid = Grid::from_vec(12, 15, data).expect("shape");
            let total: i64 = outline(&grid).iter().map(|r| area2(r)).sum();
            assert_eq!(total, 2 * count, "seed {seed}");
        }
    }

    #[test]
    fn empty_mask_has_no_rings() {
        assert_eq!(outline(&Grid::filled(3, 3, false)).len(), 0);
    }
}
