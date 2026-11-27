//! Room shapes (spec §5.7) as pure functions on one room's local mask.
//!
//! `room` is the room's cells (its blocks minus their walls) inside the room's bounding
//! rectangle: row 0, column 0 is the rectangle's north-west cell. For a 1×1 block the rectangle
//! is the room. Every shape keeps only cells of `room`.

use crate::grid::Grid;
use crate::kernels::labels::{Connectivity, labels};
use crate::kernels::paths::{Directions, path_to, shortest_paths};

/// `rect`: every cell of the room.
#[must_use]
pub fn rect(room: &Grid<bool>) -> Grid<bool> {
    room.clone()
}

/// `round`: the integer ellipse inscribed in the rectangle.
///
/// In doubled coordinates the `h` x `w` rectangle spans `0..2h` x `0..2w`, a cell `(r, c)` has
/// its centre at `(2r + 1, 2c + 1)`, and the ellipse has semi-axes `h` and `w` about `(h, w)`.
/// A room cell stays when `(2r + 1 - h)^2 * w^2 + (2c + 1 - w)^2 * h^2 <= h^2 * w^2`, all in
/// integers.
#[must_use]
pub fn round(room: &Grid<bool>) -> Grid<bool> {
    let (h, w) = (wide(room.rows()), wide(room.cols()));
    keep(room, |r, c| {
        let (y, x) = (2 * wide(r) + 1 - h, 2 * wide(c) + 1 - w);
        y * y * w * w + x * x * h * h <= h * h * w * w
    })
}

/// `ragged`: a room cell in the rectangle's outer two rings is dropped where `noise` exceeds `t`;
/// then only the 4-connected component holding `anchor` stays. `anchor` itself is never
/// dropped, so a room whose cells all lie in the outer rings keeps at least it. `noise` is the
/// rectangle's part of the run's noise; a missing value drops nothing.
#[must_use]
pub fn ragged(room: &Grid<bool>, noise: &Grid<f64>, t: f64, anchor: (usize, usize)) -> Grid<bool> {
    let (h, w) = (room.rows(), room.cols());
    let eroded = keep(room, |r, c| {
        let ring = r.min(c).min(h - 1 - r).min(w - 1 - c);
        let high = noise
            .as_slice()
            .get(noise.index(r, c))
            .is_some_and(|&v| v > t);
        (r, c) == anchor || ring >= 2 || !high
    });
    if anchor.0 >= h || anchor.1 >= w || !eroded.get(anchor.0, anchor.1) {
        return Grid::filled(h, w, false);
    }
    let (lab, _) = labels(&eroded, Connectivity::Four);
    let home = lab.get(anchor.0, anchor.1);
    lab.map(|v| v == home)
}

/// `passage`: a strip `width` cells wide through the middle of the rectangle, along its longer
/// axis (along the rows on a square). The strip starts `(across - width) / 2` cells in.
#[must_use]
pub fn passage(room: &Grid<bool>, width: usize) -> Grid<bool> {
    let along_rows = room.cols() >= room.rows();
    let across = if along_rows { room.rows() } else { room.cols() };
    let first = across.saturating_sub(width) / 2;
    keep(room, |r, c| {
        let i = if along_rows { r } else { c };
        i >= first && i < first + width
    })
}

/// The room centre: the cell of `mask` nearest to the mean of its cells, by squared distance in
/// integers (both scaled by the cell count), ties to the lowest `(row, col)`. `None` for an
/// empty mask.
#[must_use]
pub fn centre(mask: &Grid<bool>) -> Option<(usize, usize)> {
    let cells: Vec<(usize, usize)> = (0..mask.len())
        .filter(|&i| mask.as_slice()[i])
        .map(|i| mask.cell(i))
        .collect();
    let n = wide(cells.len());
    let sum_r: i128 = cells.iter().map(|&(r, _)| wide(r)).sum();
    let sum_c: i128 = cells.iter().map(|&(_, c)| wide(c)).sum();
    let dist = |&(r, c): &(usize, usize)| {
        let (y, x) = (n * wide(r) - sum_r, n * wide(c) - sum_c);
        y * y + x * x
    };
    // `min_by_key` keeps the first of equal keys, and the cells are row-major.
    cells.iter().min_by_key(|cell| dist(cell)).copied()
}

/// Carves a 4-connected line of room cells into `mask` from `from` to `to`: first along one axis
/// (the rows when `rows_first`, the door's axis), then the other. When that line leaves `room`,
/// the other order is tried, then the shortest 4-connected walk inside `room` (P08's
/// `shortest_paths`, which only a hall that is not a rectangle needs). Nothing is carved when
/// `from` or `to` is not a room cell.
pub fn carve(
    mask: &mut Grid<bool>,
    room: &Grid<bool>,
    from: (usize, usize),
    to: (usize, usize),
    rows_first: bool,
) {
    let inside = |&(r, c): &(usize, usize)| r < room.rows() && c < room.cols() && room.get(r, c);
    if !inside(&from) || !inside(&to) {
        return;
    }
    let path = [
        straight(from, to, rows_first),
        straight(from, to, !rows_first),
    ]
    .into_iter()
    .find(|p| p.iter().all(inside))
    .or_else(|| walk(room, from, to));
    for (r, c) in path.unwrap_or_default() {
        mask.set(r, c, true);
    }
}

/// The cells of the room for which `rule(row, col)` holds.
fn keep(room: &Grid<bool>, rule: impl Fn(usize, usize) -> bool) -> Grid<bool> {
    let mut out = room.clone();
    for i in 0..room.len() {
        let (r, c) = room.cell(i);
        out.as_mut_slice()[i] = room.as_slice()[i] && rule(r, c);
    }
    out
}

/// A coordinate or count as a wide signed integer for exact products.
fn wide(v: usize) -> i128 {
    i128::try_from(v).unwrap_or(i128::MAX)
}

/// The L-shaped line from `from` to `to`, both included.
fn straight(from: (usize, usize), to: (usize, usize), rows_first: bool) -> Vec<(usize, usize)> {
    let mut at = from;
    let mut cells = vec![at];
    for leg in [rows_first, !rows_first] {
        loop {
            let (cur, goal) = if leg {
                (&mut at.0, to.0)
            } else {
                (&mut at.1, to.1)
            };
            if *cur == goal {
                break;
            }
            *cur = if *cur < goal { *cur + 1 } else { *cur - 1 };
            cells.push(at);
        }
    }
    cells
}

/// The shortest 4-connected walk inside `room` from `from` to `to`.
fn walk(
    room: &Grid<bool>,
    from: (usize, usize),
    to: (usize, usize),
) -> Option<Vec<(usize, usize)>> {
    let (rows, cols) = (room.rows(), room.cols());
    let ok = |i: usize| room.as_slice().get(i).copied().unwrap_or(false);
    let mut cost = |a: usize, b: usize| {
        let ((ra, ca), (rb, cb)) = (room.cell(a), room.cell(b));
        let orthogonal = ra.abs_diff(rb) + ca.abs_diff(cb) == 1;
        (orthogonal && ok(b)).then_some(1.0)
    };
    let tree = shortest_paths(
        rows,
        cols,
        &[room.index(from.0, from.1)],
        Directions::Eight,
        &mut cost,
        &mut |_| Vec::new(),
        &|| false,
    )
    .ok()?;
    let cells = path_to(&tree, room.index(to.0, to.1))?;
    Some(cells.into_iter().map(|i| room.cell(i)).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn full(h: usize, w: usize) -> Grid<bool> {
        Grid::filled(h, w, true)
    }

    fn count(m: &Grid<bool>) -> usize {
        m.as_slice().iter().filter(|&&v| v).count()
    }

    // AC: CE-80.1
    #[test]
    fn round_drops_the_corners_and_keeps_the_middle() {
        let m = round(&full(7, 9));
        assert!(!m.get(0, 0) && !m.get(0, 8) && !m.get(6, 0) && !m.get(6, 8));
        assert!(m.get(3, 4) && m.get(0, 4) && m.get(3, 0) && m.get(3, 8));
        // Symmetric under both mirrors.
        for r in 0..7 {
            for c in 0..9 {
                assert_eq!(m.get(r, c), m.get(6 - r, c));
                assert_eq!(m.get(r, c), m.get(r, 8 - c));
            }
        }
        // A 2x2 room keeps every cell.
        assert_eq!(count(&round(&full(2, 2))), 4);
        // Only room cells are kept.
        let mut notched = full(5, 5);
        notched.set(2, 2, false);
        assert!(!round(&notched).get(2, 2));
    }

    // AC: CE-80.1
    #[test]
    fn ragged_erodes_only_the_outer_two_rings_and_keeps_the_anchor_component() {
        let room = full(7, 7);
        let high = Grid::filled(7, 7, 1.0);
        let m = ragged(&room, &high, 0.0, (3, 3));
        // Everything in the outer two rings goes; the inner 3x3 stays.
        assert_eq!(count(&m), 9);
        assert!(m.get(2, 2) && m.get(4, 4) && !m.get(1, 3));
        let low = Grid::filled(7, 7, -1.0);
        assert_eq!(ragged(&room, &low, 0.0, (3, 3)), room);
        // In a 4x4 room every cell lies in the outer rings: the anchor survives alone.
        let small = ragged(&full(4, 4), &Grid::filled(4, 4, 1.0), 0.0, (1, 1));
        assert_eq!(count(&small), 1);
        assert!(small.get(1, 1));
        // A kept cell cut off from the anchor is dropped.
        let mut noise = Grid::filled(7, 7, -1.0);
        for c in 0..7 {
            noise.set(1, c, 1.0);
        }
        let cut = ragged(&room, &noise, 0.0, (3, 3));
        assert!(!cut.get(0, 3), "row 0 is cut off by the dropped row 1");
        assert!(cut.get(3, 0));
    }

    // AC: CE-80.1
    #[test]
    fn passage_is_a_middle_strip_along_the_longer_axis() {
        let wide = passage(&full(7, 10), 2);
        assert_eq!(count(&wide), 20);
        assert!(wide.get(2, 0) && wide.get(3, 9) && !wide.get(4, 0) && !wide.get(1, 0));
        let tall = passage(&full(10, 6), 2);
        assert_eq!(count(&tall), 20);
        assert!(tall.get(0, 2) && tall.get(9, 3) && !tall.get(0, 1));
        assert_eq!(count(&passage(&full(3, 5), 9)), 15);
    }

    // AC: CE-80.1
    #[test]
    fn centre_is_the_cell_nearest_the_mean_with_ties_low() {
        assert_eq!(centre(&full(5, 5)), Some((2, 2)));
        // Even sizes: four cells tie; the lowest (row, col) wins.
        assert_eq!(centre(&full(4, 6)), Some((1, 2)));
        assert_eq!(centre(&Grid::filled(3, 3, false)), None);
        // An L: the mean lies off the cells.
        let mut l = Grid::filled(5, 5, false);
        for i in 0..5 {
            l.set(4, i, true);
            l.set(i, 0, true);
        }
        // Mean (26/9, 10/9): (3, 0) and (4, 1) both lie at 101/81; (3, 0) is lower.
        assert_eq!(centre(&l), Some((3, 0)));
    }

    // AC: CE-80.1
    #[test]
    fn carve_draws_the_door_axis_first() {
        let room = full(6, 6);
        let mut m = Grid::filled(6, 6, false);
        carve(&mut m, &room, (0, 1), (3, 4), true);
        for r in 0..=3 {
            assert!(m.get(r, 1));
        }
        for c in 1..=4 {
            assert!(m.get(3, c));
        }
        assert_eq!(count(&m), 7);
        let mut n = Grid::filled(6, 6, false);
        carve(&mut n, &room, (1, 0), (4, 3), false);
        assert!(n.get(1, 3) && n.get(4, 3) && !n.get(4, 0));
    }

    // AC: CE-80.1
    #[test]
    fn carve_stays_inside_the_room() {
        // An L-shaped room: the straight lines both leave it, so the carve walks inside it.
        let mut room = Grid::filled(6, 6, false);
        for i in 0..6 {
            room.set(5, i, true);
            room.set(i, 0, true);
        }
        let mut m = Grid::filled(6, 6, false);
        carve(&mut m, &room, (0, 0), (5, 5), true);
        for r in 0..6 {
            for c in 0..6 {
                assert!(!m.get(r, c) || room.get(r, c));
            }
        }
        assert!(m.get(0, 0) && m.get(5, 5) && m.get(5, 0));
        // A start outside the room carves nothing.
        let mut none = Grid::filled(6, 6, false);
        carve(&mut none, &room, (2, 2), (5, 5), true);
        assert_eq!(count(&none), 0);
    }
}
