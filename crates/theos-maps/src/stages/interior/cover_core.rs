//! The cover core (spec §5.8): pillars and rubble that break long sightlines without cutting
//! the interior in two.
//!
//! 0.3.0's `cover` stage in integers. A ring of pillars comes first in round halls that are wide
//! enough; then, while the longest straight run of free floor exceeds the limit, one pillar (in a
//! room) or one rubble cell (in a corridor) goes near the middle of that run. A placement that
//! splits the walkable cells is undone.

use crate::grid::Grid;
use crate::kernels::labels::{Connectivity, labels};
use crate::kernels::num::{to_f64, to_index, to_u32, to_usize};
use crate::rooms::{RoomGraph, Shape};
use crate::stages::interior::space_core::SpaceOut;

/// The parameters of the cover core.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CoverParams {
    /// The longest sightline allowed, in metres.
    pub max_sightline_m: f64,
    /// The most items placed, ring pillars included (default 120).
    pub max_items: u32,
    /// The smaller side, in metres, from which a round hall gets a ring of pillars.
    pub pillar_ring_m: f64,
}

/// The directions of a run, in tie-break order: along the rows, along the columns, down and to
/// the east (↘), down and to the west (↙).
const DIRECTIONS: [(isize, isize); 4] = [(0, 1), (1, 0), (1, 1), (1, -1)];

/// The offsets from the middle of a run that are tried, in order.
const OFFSETS: [isize; 9] = [0, 2, -2, 4, -4, 6, -6, 8, -8];

/// Cosine and sine of k x 36 degrees, times 1000 and rounded: the ring's ten pillars.
const RING: [(i64, i64); 10] = [
    (1000, 0),
    (809, 588),
    (309, 951),
    (-309, 951),
    (-809, 588),
    (-1000, 0),
    (-809, -588),
    (-309, -951),
    (309, -951),
    (809, -588),
];

/// A straight run of `true` cells.
#[derive(Debug, Clone, Copy)]
struct Run {
    /// The number of cells.
    len: u32,
    /// The step between two cells.
    step: (isize, isize),
    /// The first cell.
    start: (usize, usize),
}

/// The longest run of `true` cells of `free`; the first found when equal, by direction in the
/// order of [`DIRECTIONS`], then by its first cell in row-major order.
fn longest_run(free: &Grid<bool>) -> Option<Run> {
    let mut best: Option<Run> = None;
    for step in DIRECTIONS {
        for i in 0..free.len() {
            if !free.as_slice()[i] {
                continue;
            }
            let (r, c) = free.cell(i);
            let before = free.neighbour_at(r, c, (-step.0, -step.1));
            if before.is_some_and(|j| free.as_slice()[j]) {
                continue;
            }
            let mut len = 1_u32;
            let mut at = (r, c);
            while let Some(j) = free.neighbour_at(at.0, at.1, step)
                && free.as_slice()[j]
            {
                len += 1;
                at = free.cell(j);
            }
            if best.is_none_or(|b| len > b.len) {
                best = Some(Run {
                    len,
                    step,
                    start: (r, c),
                });
            }
        }
    }
    best
}

/// The length in cells of the longest straight run of `true` cells of `free`, along rows,
/// columns and both diagonals (0 when there is none). A diagonal run counts its cells.
#[must_use]
pub fn longest_sightline_cells(free: &Grid<bool>) -> u32 {
    longest_run(free).map_or(0, |r| r.len)
}

/// What a placement reads: the floor, the rooms, the keep-out zones and the component count.
struct Floor<'a> {
    /// The interior's rasters.
    space: &'a SpaceOut,
    /// Cells where nothing may be placed.
    keep: Grid<bool>,
    /// The number of components of the floor before any cover.
    parts: u32,
}

impl Floor<'_> {
    /// The floor minus `cover`.
    fn open(&self, cover: &Grid<bool>) -> Grid<bool> {
        let mut open = self.space.walkable.clone();
        for (o, &c) in open.as_mut_slice().iter_mut().zip(cover.as_slice()) {
            *o = *o && !c;
        }
        open
    }

    /// Whether the floor minus `cover` has the components the floor had.
    fn connected(&self, cover: &Grid<bool>) -> bool {
        labels(&self.open(cover), Connectivity::Four).1 == self.parts
    }

    /// The cells a pillar with its corner at `(r, c)` covers: the cell and its east, south and
    /// south-east cells where they are free floor of the same room.
    fn pillar(&self, cover: &Grid<bool>, (r, c): (usize, usize)) -> Vec<usize> {
        let rooms = &self.space.rooms;
        let room = rooms.get(r, c);
        [(0, 0), (0, 1), (1, 0), (1, 1)]
            .iter()
            .filter_map(|&d| rooms.neighbour_at(r, c, d))
            .filter(|&i| {
                rooms.as_slice()[i] == room
                    && self.space.walkable.as_slice()[i]
                    && !cover.as_slice()[i]
            })
            .collect()
    }

    /// The cells covering `(r, c)`: a pillar in a room, one rubble cell elsewhere on the floor.
    fn item(&self, cover: &Grid<bool>, (r, c): (usize, usize)) -> Vec<usize> {
        let i = self.space.rooms.index(r, c);
        if self.space.rooms.as_slice()[i] > 0 && !self.space.corridors.as_slice()[i] {
            self.pillar(cover, (r, c))
        } else if self.space.walkable.as_slice()[i] && !cover.as_slice()[i] {
            vec![i]
        } else {
            Vec::new()
        }
    }

    /// Puts `cells` into `cover` when that keeps the floor connected, else leaves `cover` as it
    /// was. Whether it was put; nothing to put counts as not put.
    fn add(&self, cover: &mut Grid<bool>, cells: &[usize]) -> bool {
        if cells.is_empty() {
            return false;
        }
        for &i in cells {
            cover.as_mut_slice()[i] = true;
        }
        if self.connected(cover) {
            return true;
        }
        for &i in cells {
            cover.as_mut_slice()[i] = false;
        }
        false
    }
}

/// The keep-out zones: 5x5 around every door centre and 3x3 around every room centre.
fn keep_out(space: &SpaceOut) -> Grid<bool> {
    let mut keep = Grid::filled(space.walkable.rows(), space.walkable.cols(), false);
    let doors = space.doors.0.iter().map(|d| (d.centre, 2_isize));
    let rooms = space.centres.iter().map(|&c| (c, 1_isize));
    for ((r, c), reach) in doors.chain(rooms) {
        for dr in -reach..=reach {
            for dc in -reach..=reach {
                if let Some(i) = keep.neighbour_at(to_usize(r), to_usize(c), (dr, dc)) {
                    keep.as_mut_slice()[i] = true;
                }
            }
        }
    }
    keep
}

/// `v` x `scale` / 1 000 000, rounded half away from zero.
fn scaled(v: i64, scale: i64) -> i64 {
    let n = v * scale;
    (2 * n + n.signum() * 1_000_000) / 2_000_000
}

/// The height and width of the bounding box of the cells of `rooms` that hold `id`.
fn room_size(rooms: &Grid<u32>, id: u32) -> Option<(usize, usize)> {
    let mut found: Option<(usize, usize, usize, usize)> = None;
    for i in (0..rooms.len()).filter(|&i| rooms.as_slice()[i] == id) {
        let (r, c) = rooms.cell(i);
        found = Some(found.map_or((r, c, r, c), |(r0, c0, r1, c1)| {
            (r0.min(r), c0.min(c), r1.max(r), c1.max(c))
        }));
    }
    found.map(|(r0, c0, r1, c1)| (r1 - r0 + 1, c1 - c0 + 1))
}

/// The ring of pillars in every round room whose smaller side is at least `p.pillar_ring_m`:
/// ten pillars at k x 36 degrees on a circle of 0.33 of that side. Returns the items placed; the
/// whole ring is dropped (nothing placed) when it splits the floor.
fn ring(
    floor: &Floor<'_>,
    g: &RoomGraph,
    cell_m: f64,
    p: &CoverParams,
    cover: &mut Grid<bool>,
) -> u32 {
    let mut trial = cover.clone();
    let mut items = 0;
    for (k, room) in g.rooms.iter().enumerate() {
        let Some((high, wide)) = room_size(&floor.space.rooms, to_u32(k + 1)) else {
            continue;
        };
        let side = high.min(wide);
        if room.shape != Shape::Round || to_f64(side) * cell_m < p.pillar_ring_m {
            continue;
        }
        let Some(&(cy, cx)) = floor.space.centres.get(k) else {
            continue;
        };
        let radius = i64::try_from(side).unwrap_or(0) * 330;
        for (cos, sin) in RING {
            let y = i64::from(cy) + scaled(radius, sin);
            let x = i64::from(cx) + scaled(radius, cos);
            let (Ok(y), Ok(x)) = (usize::try_from(y), usize::try_from(x)) else {
                continue;
            };
            if y >= trial.rows() || x >= trial.cols() {
                continue;
            }
            let cells = floor.pillar(&trial, (y, x));
            if !cells.is_empty() {
                for i in cells {
                    trial.as_mut_slice()[i] = true;
                }
                items += 1;
            }
        }
    }
    if floor.connected(&trial) {
        *cover = trial;
        items
    } else {
        0
    }
}

/// Places cover on the interior: a pillar ring in round halls, then pillars and rubble at the
/// middle of the longest sightline until no run of free floor (`walkable` minus cover) is longer
/// than `p.max_sightline_m` or `p.max_items` items are placed. Returns the cover raster, a
/// subset of `walkable`, and the number of items (ring pillars included).
///
/// Each step takes the longest run (ties: rows, columns, then the two diagonals, then its first
/// cell) and tries the cells at offsets 0, +2, -2 ... -8 from its middle, skipping the keep-out
/// zones. The first placement that leaves the floor in as many 4-connected components as it had
/// stays. A run where nothing fits is given up and not chosen again.
#[must_use]
pub fn place_cover(
    space: &SpaceOut,
    g: &RoomGraph,
    cell_m: f64,
    p: &CoverParams,
) -> (Grid<bool>, u32) {
    let (rows, cols) = (space.walkable.rows(), space.walkable.cols());
    let floor = Floor {
        space,
        keep: keep_out(space),
        parts: labels(&space.walkable, Connectivity::Four).1,
    };
    let mut cover = Grid::filled(rows, cols, false);
    let mut items = ring(&floor, g, cell_m, p, &mut cover);
    let limit = to_u32(to_index(p.max_sightline_m / cell_m, usize::MAX));
    let mut gave_up = Grid::filled(rows, cols, false);
    while items < p.max_items {
        let mut free = floor.open(&cover);
        for (f, &given) in free.as_mut_slice().iter_mut().zip(gave_up.as_slice()) {
            *f = *f && !given;
        }
        let Some(run) = longest_run(&free).filter(|r| r.len > limit) else {
            break;
        };
        let cells: Vec<(usize, usize)> = (0..to_usize(run.len))
            .filter_map(|k| {
                let n = isize::try_from(k).ok()?;
                let row = run.start.0.checked_add_signed(run.step.0 * n)?;
                let col = run.start.1.checked_add_signed(run.step.1 * n)?;
                Some((row, col))
            })
            .collect();
        let mid = isize::try_from(cells.len() / 2).unwrap_or(0);
        let placed = OFFSETS.iter().any(|&off| {
            let Some(&(r, c)) = usize::try_from(mid + off).ok().and_then(|k| cells.get(k)) else {
                return false;
            };
            if floor.keep.get(r, c) {
                return false;
            }
            let item = floor.item(&cover, (r, c));
            floor.add(&mut cover, &item)
        });
        if placed {
            items += 1;
        } else {
            for (r, c) in cells {
                gave_up.set(r, c, true);
            }
        }
    }
    (cover, items)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rooms::{Cell, Connection, LatticeRef, Passage, Role, Room};
    use crate::stages::interior::space_core::{Door, Doors};

    /// A raster with the given floor rectangles `(r0, r1, c0, c1, corridor)`; a rectangle that
    /// is not a corridor is room number position + 1.
    fn space(rows: usize, cols: usize, rects: &[(usize, usize, usize, usize, bool)]) -> SpaceOut {
        let mut out = SpaceOut {
            rooms: Grid::filled(rows, cols, 0),
            walkable: Grid::filled(rows, cols, false),
            walls: Grid::filled(rows, cols, false),
            corridors: Grid::filled(rows, cols, false),
            doors: Doors(Vec::new()),
            centres: Vec::new(),
        };
        for (k, &(r0, r1, c0, c1, corridor)) in rects.iter().enumerate() {
            for r in r0..r1 {
                for c in c0..c1 {
                    out.walkable.set(r, c, true);
                    if corridor {
                        out.corridors.set(r, c, true);
                    } else {
                        out.rooms.set(r, c, to_u32(k + 1));
                    }
                }
            }
            out.centres.push((
                to_u32(usize::midpoint(r0, r1)),
                to_u32(usize::midpoint(c0, c1)),
            ));
        }
        out
    }

    fn graph(shapes: &[Shape]) -> RoomGraph {
        RoomGraph {
            rooms: shapes
                .iter()
                .map(|&shape| Room {
                    ty: "hall".to_owned(),
                    role: Role::Path,
                    cells: vec![Cell { row: 0, col: 0 }],
                    shape,
                    keys: Vec::new(),
                    locked: false,
                })
                .collect(),
            connections: Vec::<Connection>::new(),
            lattice: LatticeRef { rows: 1, cols: 1 },
        }
    }

    fn door(at: (u32, u32)) -> Door {
        Door {
            connection: 0,
            cells: vec![at],
            rooms: (0, 1),
            kind: Passage::Open,
            centre: at,
        }
    }

    fn params(limit: f64, max_items: u32) -> CoverParams {
        CoverParams {
            max_sightline_m: limit,
            max_items,
            pillar_ring_m: 14.0,
        }
    }

    fn open(space: &SpaceOut, cover: &Grid<bool>) -> Grid<bool> {
        let mut open = space.walkable.clone();
        for (o, &c) in open.as_mut_slice().iter_mut().zip(cover.as_slice()) {
            *o = *o && !c;
        }
        open
    }

    fn parts(space: &SpaceOut, cover: &Grid<bool>) -> u32 {
        labels(&open(space, cover), Connectivity::Four).1
    }

    // AC: CE-80.2
    #[test]
    fn long_hall_gets_pillars_until_the_limit() {
        let space = space(16, 30, &[(1, 15, 1, 29, false)]);
        let g = graph(&[Shape::Rect]);
        let (cover, items) = place_cover(&space, &g, 1.0, &params(8.0, 120));
        assert!(items > 0 && items < 120, "items {items}");
        assert!(longest_sightline_cells(&open(&space, &cover)) <= 8);
        assert_eq!(parts(&space, &cover), 1);
        for i in 0..cover.len() {
            assert!(!cover.as_slice()[i] || space.walkable.as_slice()[i]);
        }
        // Pillars are 2x2.
        assert!(cover.as_slice().iter().filter(|&&c| c).count() >= 4);
    }

    // AC: CE-80.2
    #[test]
    fn cover_never_disconnects() {
        // Two halls joined by a one-cell corridor and a two-cell one.
        let mut space = space(
            14,
            40,
            &[
                (1, 13, 1, 17, false),
                (1, 13, 23, 39, false),
                (6, 7, 17, 23, true),
                (11, 13, 17, 23, true),
            ],
        );
        space.doors = Doors(vec![door((6, 20))]);
        let g = graph(&[Shape::Rect, Shape::Rect, Shape::Rect, Shape::Rect]);
        for limit in [3.0, 5.0, 8.0] {
            let (cover, _) = place_cover(&space, &g, 1.0, &params(limit, 120));
            assert_eq!(parts(&space, &cover), 1, "limit {limit}");
            // The door's keep-out square stays clear.
            for r in 4..=8 {
                for c in 18..=22 {
                    assert!(!cover.get(r, c), "({r}, {c})");
                }
            }
        }
    }

    // AC: CE-80.2
    #[test]
    fn pillar_ring_dropped_when_it_cuts_a_door() {
        // A round 20 x 20 hall. Its pillar at angle 0 lands on (10, 17), the two-cell gap in a
        // wall that closes off the east strip, where the door is.
        let build = |walled: bool| {
            let mut space = space(22, 22, &[(1, 21, 1, 21, false)]);
            space.centres = vec![(10, 10)];
            if walled {
                for r in (1..21).filter(|&r| r != 10 && r != 11) {
                    space.walkable.set(r, 17, false);
                    space.rooms.set(r, 17, 0);
                }
                space.doors = Doors(vec![door((5, 19))]);
            }
            space
        };
        let g = graph(&[Shape::Round]);
        let p = params(1000.0, 120);
        let (cover, items) = place_cover(&build(true), &g, 1.0, &p);
        assert_eq!(items, 0);
        assert!(cover.as_slice().iter().all(|&c| !c));
        let (cover, items) = place_cover(&build(false), &g, 1.0, &p);
        assert_eq!(items, 10);
        assert!(cover.as_slice().iter().any(|&c| c));
        // A hall below the ring size gets no ring.
        let small = CoverParams {
            pillar_ring_m: 21.0,
            ..p
        };
        assert_eq!(place_cover(&build(false), &g, 1.0, &small).1, 0);
    }

    // AC: CE-80.2
    #[test]
    fn rubble_only_interior_stops_at_max_items_without_disconnecting() {
        let space = space(5, 62, &[(1, 4, 1, 61, true)]);
        let g = graph(&[Shape::Passage]);
        let (cover, items) = place_cover(&space, &g, 1.0, &params(4.0, 5));
        assert_eq!(items, 5);
        assert_eq!(cover.as_slice().iter().filter(|&&c| c).count(), 5);
        assert_eq!(parts(&space, &cover), 1);
        // A one-cell corridor takes no rubble: every placement would cut it.
        let thin = self::space(3, 62, &[(1, 2, 1, 61, true)]);
        let (cover, items) = place_cover(&thin, &g, 1.0, &params(4.0, 5));
        assert_eq!(items, 0);
        assert!(cover.as_slice().iter().all(|&c| !c));
    }

    // AC: CE-80.2
    #[test]
    fn diagonal_sightline_counts() {
        let mut grid = Grid::filled(10, 10, false);
        for k in 0..7 {
            grid.set(k, k, true);
        }
        assert_eq!(longest_sightline_cells(&grid), 7);
        let mut back = Grid::filled(10, 10, false);
        for k in 0..6 {
            back.set(k, 9 - k, true);
        }
        assert_eq!(longest_sightline_cells(&back), 6);
        assert_eq!(longest_sightline_cells(&Grid::filled(3, 3, false)), 0);
        assert_eq!(longest_sightline_cells(&Grid::filled(3, 4, true)), 4);
    }
}
