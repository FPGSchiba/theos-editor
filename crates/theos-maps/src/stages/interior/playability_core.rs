//! The playability core (spec §5.9): the checks that prove an interior playable (CE-80.2), its
//! metrics and its critical route.
//!
//! Every check and every distance runs on the *free floor*, `walkable` minus `cover`. Findings
//! are data: [`analyse`] collects all of them, and the stage fails the run when there is one.

use std::any::Any;

use serde::{Deserialize, Serialize};

use crate::grid::Grid;
use crate::kernels::KernelError;
use crate::kernels::labels::{Connectivity, labels};
use crate::kernels::paths::{Directions, PathTree, shortest_paths};
use crate::rooms::measure::{articulation_points, cycles, dead_end_pct};
use crate::rooms::solve::{Unsolvable, critical_route, fixed_point, solvable, unsafe_drops};
use crate::rooms::{KeyId, Passage, Role, RoomGraph, RoomId};
use crate::stages::interior::cover_core::longest_sightline_cells;
use crate::stages::interior::space_core::{Door, SpaceOut};
use crate::world::{ValueType, WorldValue};

/// The check a finding comes from; written in snake case.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Code {
    /// A room of the graph has no cell in `rooms` (check 1).
    UncarvedRoom,
    /// The free floor is not one piece, or a room lies outside the start's piece (check 2).
    Disconnected,
    /// The lock-and-key fixed point does not reach a room (check 3).
    UnreachableRoom,
    /// A drop can strand the party out of the goal's reach (check 4).
    UnsafeDrop,
    /// A connection has no door of its rooms, or a door names no connection (check 5).
    DoorMismatch,
}

impl Code {
    /// The code in snake case, as it starts a finding's message.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::UncarvedRoom => "uncarved_room",
            Self::Disconnected => "disconnected",
            Self::UnreachableRoom => "unreachable_room",
            Self::UnsafeDrop => "unsafe_drop",
            Self::DoorMismatch => "door_mismatch",
        }
    }
}

/// The `analysis` world value: the metrics of §5.9, the critical route and the findings.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Analysis {
    /// The number of rooms.
    pub rooms: f64,
    /// The number of connections.
    pub connections: f64,
    /// The number of independent cycles of the graph.
    pub cycles: f64,
    /// The number of locked passages.
    pub locks: f64,
    /// The number of drops.
    pub one_way: f64,
    /// The number of articulation points of the graph.
    pub chokepoints: f64,
    /// The share of rooms with one connection, in percent.
    pub dead_end_pct: f64,
    /// Walkable cells over envelope cells, in percent.
    pub footprint_used_pct: f64,
    /// Cells of rooms on two or more lattice cells over walkable cells, in percent.
    pub hall_share_pct: f64,
    /// The length of the critical route in metres; 0 when there is none.
    pub critical_path_m: f64,
    /// The longest straight run of free floor, in metres.
    pub longest_sightline_m: f64,
    /// The free-floor distance from the start's centre to the nearest exit room's centre;
    /// absent when the graph has no exit room or none is reached.
    pub start_to_exit_min_m: Option<f64>,
    /// The free-floor distance from the goal's centre to the nearest exit room's centre;
    /// absent when the graph has no exit room or none is reached.
    pub goal_to_exit_min_m: Option<f64>,
    /// The rooms of the critical route from start to goal, revisits included; empty when the
    /// goal cannot be reached.
    pub route: Vec<RoomId>,
    /// The route as raster `(row, col)` points: the start's centre, then per connection crossed
    /// its door's centre and the next room's centre, ending at the goal's centre. Its legs are
    /// the ones `critical_path_m` measures (centre -> door -> centre, spec §5.9).
    pub route_points: Vec<(u32, u32)>,
    /// The failed checks, each with a message that starts with its code.
    pub findings: Vec<(Code, String)>,
}

impl WorldValue for Analysis {
    fn type_name(&self) -> &'static str {
        Self::NAME
    }
    fn canonical_json(&self) -> Result<serde_json::Value, theos_map_format::FormatError> {
        theos_map_format::json::to_checked_value(self)
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

impl ValueType for Analysis {
    const NAME: &'static str = "analysis";
}

/// Analyses an interior (spec §5.9): runs the five checks, measures it and finds its critical
/// route.
///
/// `cover` is taken off `walkable` to give the free floor. `cell_m` is the size of a raster
/// cell in metres. `frame_cells` is the frame in raster cells, `(rows, cols)`, the size of every
/// raster given. The checks run in the order of [`Code`]; every failure becomes a finding.
///
/// # Errors
/// [`KernelError`] when a raster search fails (a frame too large for the path kernel).
pub fn analyse(
    g: &RoomGraph,
    s: &SpaceOut,
    cover: &Grid<bool>,
    envelope: &Grid<bool>,
    cell_m: f64,
    frame_cells: (usize, usize),
) -> Result<Analysis, KernelError> {
    let mut free = s.walkable.clone();
    for (f, &c) in free.as_mut_slice().iter_mut().zip(cover.as_slice()) {
        *f &= !c;
    }
    let mut findings = Vec::new();
    let carved = carved_cells(g, s);
    uncarved(&carved, &mut findings);
    disconnected(g, s, &free, &carved, &mut findings);
    unreachable(g, &mut findings);
    unsafe_drop(g, &mut findings);
    door_mismatch(g, s, &mut findings);

    let mut dist = Distances {
        free: &free,
        frame: frame_cells,
        cell_m,
        centres: &s.centres,
        trees: vec![None; g.rooms.len()],
    };
    let critical = route(g, s, &mut dist)?;
    let start = role_room(g, Role::Start);
    let goal = role_room(g, Role::Goal);
    let start_to_exit_min_m = to_exit(g, &mut dist, start)?;
    let goal_to_exit_min_m = to_exit(g, &mut dist, goal)?;

    let walkable = s.walkable.as_slice().iter().filter(|&&w| w).count();
    let envelope_cells = envelope.as_slice().iter().filter(|&&e| e).count();
    let hall_cells: usize = carved
        .iter()
        .enumerate()
        .filter(|&(i, _)| g.rooms[i].cells.len() >= 2)
        .map(|(_, &n)| n)
        .sum();
    let count_of =
        |pred: fn(&Passage) -> bool| count(g.connections.iter().filter(|c| pred(&c.kind)).count());
    Ok(Analysis {
        rooms: count(g.rooms.len()),
        connections: count(g.connections.len()),
        cycles: count(cycles(g)),
        locks: count_of(|p| matches!(p, Passage::Locked(_))),
        one_way: count_of(|p| matches!(p, Passage::Drop { .. })),
        chokepoints: count(articulation_points(g).len()),
        dead_end_pct: dead_end_pct(g),
        footprint_used_pct: pct(walkable, envelope_cells),
        hall_share_pct: pct(hall_cells, walkable),
        critical_path_m: critical.length_m,
        longest_sightline_m: f64::from(longest_sightline_cells(&free)) * cell_m,
        start_to_exit_min_m,
        goal_to_exit_min_m,
        route: critical.rooms,
        route_points: critical.points,
        findings,
    })
}

/// A finding with its code in front of the message.
fn finding(code: Code, message: &str) -> (Code, String) {
    (code, format!("{}: {message}", code.as_str()))
}

/// The name of room index `i` as features write it: `room-<i + 1>`.
fn room_name(i: usize) -> String {
    format!("room-{}", i + 1)
}

/// The name of the door of connection index `i`: `door-<i + 1>`.
fn door_name(i: usize) -> String {
    format!("door-{}", i + 1)
}

/// A count as a metric.
fn count(n: usize) -> f64 {
    f64::from(u32::try_from(n).unwrap_or(u32::MAX))
}

/// `100 * part / whole`, 0 for an empty whole.
fn pct(part: usize, whole: usize) -> f64 {
    if whole == 0 {
        0.0
    } else {
        count(part) * 100.0 / count(whole)
    }
}

/// The first room with `role`.
fn role_room(g: &RoomGraph, role: Role) -> Option<usize> {
    g.rooms.iter().position(|r| r.role == role)
}

/// Per room, its number of cells in the `rooms` raster.
fn carved_cells(g: &RoomGraph, s: &SpaceOut) -> Vec<usize> {
    let mut n = vec![0; g.rooms.len()];
    for &v in s.rooms.as_slice() {
        if let Some(slot) = v
            .checked_sub(1)
            .and_then(|k| usize::try_from(k).ok())
            .and_then(|k| n.get_mut(k))
        {
            *slot += 1;
        }
    }
    n
}

/// Check 1: every room has cells in `rooms`.
fn uncarved(carved: &[usize], out: &mut Vec<(Code, String)>) {
    for (i, _) in carved.iter().enumerate().filter(|&(_, &n)| n == 0) {
        out.push(finding(
            Code::UncarvedRoom,
            &format!("{} has no cell", room_name(i)),
        ));
    }
}

/// Check 2: the free floor is one 4-connected piece holding every carved room's centre and
/// cells. Names every carved room cut off from the start's piece; when the floor is in pieces
/// but every room is in the start's, names the doors outside it.
fn disconnected(
    g: &RoomGraph,
    s: &SpaceOut,
    free: &Grid<bool>,
    carved: &[usize],
    out: &mut Vec<(Code, String)>,
) {
    let (lab, pieces) = labels(free, Connectivity::Four);
    let label_at = |(r, c): (u32, u32)| {
        let (r, c) = (to_usize(r), to_usize(c));
        if r < lab.rows() && c < lab.cols() {
            lab.get(r, c)
        } else {
            0
        }
    };
    let Some(start) = role_room(g, Role::Start) else {
        return;
    };
    let home = s.centres.get(start).map_or(0, |&c| label_at(c));
    let mut cut = vec![false; g.rooms.len()];
    for (i, centre) in s.centres.iter().enumerate().take(g.rooms.len()) {
        cut[i] = home == 0 || label_at(*centre) != home;
    }
    for ((&v, &l), &f) in s
        .rooms
        .as_slice()
        .iter()
        .zip(lab.as_slice())
        .zip(free.as_slice())
    {
        if let Some(slot) = v
            .checked_sub(1)
            .and_then(|k| usize::try_from(k).ok())
            .and_then(|k| cut.get_mut(k))
            && f
            && l != home
        {
            *slot = true;
        }
    }
    let rooms: Vec<String> = (0..g.rooms.len())
        .filter(|&i| cut[i] && carved[i] > 0)
        .map(room_name)
        .collect();
    let from = room_name(start);
    if !rooms.is_empty() {
        out.push(finding(
            Code::Disconnected,
            &format!("{} cut off from {from}", rooms.join(", ")),
        ));
    } else if pieces > 1 {
        let doors: Vec<String> = s
            .doors
            .0
            .iter()
            .filter(|d| d.cells.iter().any(|&c| label_at(c) != home))
            .map(|d| door_name(d.connection))
            .collect();
        let what = if doors.is_empty() {
            format!("the floor is in {pieces} pieces")
        } else {
            format!("{} cut off from {from}", doors.join(", "))
        };
        out.push(finding(Code::Disconnected, &what));
    }
}

/// Check 3: the lock-and-key fixed point reaches every room. Each unreached room is named with
/// the keys of the locks on the frontier that are not held.
fn unreachable(g: &RoomGraph, out: &mut Vec<(Code, String)>) {
    let reach = fixed_point(g, true);
    let missing: Vec<KeyId> = match solvable(g) {
        Err(Unsolvable::UnreachableRoom { missing, .. }) => missing,
        _ => Vec::new(),
    };
    let why = if missing.is_empty() {
        "is not reached".to_owned()
    } else {
        let keys: Vec<String> = missing.iter().map(|&k| RoomGraph::key_name(k)).collect();
        format!("needs {}", keys.join(", "))
    };
    for (i, _) in reach.rooms.iter().enumerate().filter(|&(_, &r)| !r) {
        out.push(finding(
            Code::UnreachableRoom,
            &format!("{} {why}", room_name(i)),
        ));
    }
}

/// Check 4: no drop strands the party out of the goal's reach.
fn unsafe_drop(g: &RoomGraph, out: &mut Vec<(Code, String)>) {
    for i in unsafe_drops(g) {
        let c = g.connections[i];
        let Passage::Drop { from } = c.kind else {
            continue;
        };
        let to = if from == c.a { c.b } else { c.a };
        out.push(finding(
            Code::UnsafeDrop,
            &format!(
                "{} drops from {} into {}, where the goal is not reachable with the keys surely held (the start room's and the drop room's)",
                door_name(i),
                room_name(usize::from(from)),
                room_name(usize::from(to))
            ),
        ));
    }
}

/// Check 5: every connection has exactly one door of its two rooms, and every door names a
/// connection.
fn door_mismatch(g: &RoomGraph, s: &SpaceOut, out: &mut Vec<(Code, String)>) {
    for (i, c) in g.connections.iter().enumerate() {
        let n = s
            .doors
            .0
            .iter()
            .filter(|d| d.connection == i && d.rooms == (c.a, c.b))
            .count();
        if n != 1 {
            out.push(finding(
                Code::DoorMismatch,
                &format!(
                    "{} between {} and {} has {n} matching doors",
                    door_name(i),
                    room_name(usize::from(c.a)),
                    room_name(usize::from(c.b))
                ),
            ));
        }
    }
    for d in s
        .doors
        .0
        .iter()
        .filter(|d| d.connection >= g.connections.len())
    {
        out.push(finding(
            Code::DoorMismatch,
            &format!("{} names no connection", door_name(d.connection)),
        ));
    }
}

/// Free-floor distances from room centres, one search per room, made when first asked for.
struct Distances<'a> {
    /// The free floor.
    free: &'a Grid<bool>,
    /// The frame in raster cells, `(rows, cols)`.
    frame: (usize, usize),
    /// The size of a raster cell in metres.
    cell_m: f64,
    /// The room centres.
    centres: &'a [(u32, u32)],
    /// The search from each room's centre, once made.
    trees: Vec<Option<PathTree>>,
}

impl Distances<'_> {
    /// The distance in metres from `room`'s centre to `cell` on the free floor; infinite when
    /// the cell is out of reach.
    fn metres(&mut self, room: usize, cell: (u32, u32)) -> Result<f64, KernelError> {
        let (rows, cols) = self.frame;
        if self.trees.get(room).is_some_and(Option::is_none) {
            let sources: Vec<usize> = self
                .centres
                .get(room)
                .map(|&(r, c)| to_usize(r) * cols + to_usize(c))
                .into_iter()
                .collect();
            let free = self.free.as_slice();
            let step = self.cell_m;
            let mut cost = |from: usize, to: usize| {
                let diagonal = from / cols != to / cols && from % cols != to % cols;
                let open = free.get(from).copied().unwrap_or(false)
                    && free.get(to).copied().unwrap_or(false);
                (open && !diagonal).then_some(step)
            };
            let tree = shortest_paths(
                rows,
                cols,
                &sources,
                Directions::Eight,
                &mut cost,
                &mut |_| Vec::new(),
                &|| false,
            )?;
            self.trees[room] = Some(tree);
        }
        let at = to_usize(cell.0) * cols + to_usize(cell.1);
        Ok(self
            .trees
            .get(room)
            .and_then(Option::as_ref)
            .and_then(|t| t.dist.get(at).copied())
            .unwrap_or(f64::INFINITY))
    }
}

/// A critical route found by [`route`].
#[derive(Debug, Default)]
struct Route {
    /// The rooms, revisits included.
    rooms: Vec<RoomId>,
    /// The length in metres.
    length_m: f64,
    /// Every room centre on the route with the door centres between them.
    points: Vec<(u32, u32)>,
}

/// The critical route: its rooms, its length in metres and its points. Empty with length 0
/// when the goal cannot be reached (or only over floor that is not connected).
fn route(g: &RoomGraph, s: &SpaceOut, dist: &mut Distances<'_>) -> Result<Route, KernelError> {
    let door_of = |i: usize| -> Option<&Door> { s.doors.0.iter().find(|d| d.connection == i) };
    let mut failure = None;
    let mut len = |i: usize, from: RoomId| -> f64 {
        let (Some(door), Some(c)) = (door_of(i), g.connections.get(i)) else {
            return f64::INFINITY;
        };
        let to = if from == c.a { c.b } else { c.a };
        let legs = dist
            .metres(usize::from(from), door.centre)
            .and_then(|x| Ok(x + dist.metres(usize::from(to), door.centre)?));
        legs.unwrap_or_else(|e| {
            failure.get_or_insert(e);
            f64::INFINITY
        })
    };
    let found = critical_route(g, &mut len);
    if let Some(e) = failure {
        return Err(e);
    }
    let Some((rooms, total)) = found.filter(|(_, t)| t.is_finite()) else {
        return Ok(Route::default());
    };
    let centre = |r: RoomId| s.centres.get(usize::from(r)).copied().unwrap_or((0, 0));
    let mut points: Vec<(u32, u32)> = rooms.first().map(|&r| centre(r)).into_iter().collect();
    for w in rooms.windows(2) {
        let pair = (w[0].min(w[1]), w[0].max(w[1]));
        if let Some(d) = g
            .connections
            .iter()
            .position(|c| (c.a, c.b) == pair)
            .and_then(door_of)
        {
            points.push(d.centre);
        }
        points.push(centre(w[1]));
    }
    Ok(Route {
        rooms,
        length_m: total,
        points,
    })
}

/// The free-floor distance from `from`'s centre to the nearest exit room's centre; `None`
/// without a `from` room, without an exit room, or when no exit is reached.
fn to_exit(
    g: &RoomGraph,
    dist: &mut Distances<'_>,
    from: Option<usize>,
) -> Result<Option<f64>, KernelError> {
    let Some(from) = from else {
        return Ok(None);
    };
    let mut best: Option<f64> = None;
    for (i, _) in g
        .rooms
        .iter()
        .enumerate()
        .filter(|(_, r)| r.role == Role::Exit)
    {
        let Some(&centre) = dist.centres.get(i) else {
            continue;
        };
        let d = dist.metres(from, centre)?;
        if d.is_finite() && best.is_none_or(|b| d < b) {
            best = Some(d);
        }
    }
    Ok(best)
}

/// A raster coordinate as an index.
fn to_usize(v: u32) -> usize {
    usize::try_from(v).unwrap_or(usize::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::StageRng;
    use crate::rooms::lattice::{Lattice, Stagger};
    use crate::rooms::{Cell, Connection, LatticeRef, Room, Shape};
    use crate::stages::interior::space_core::{SpaceParams, build_space};

    /// The size of a raster cell in the fixtures; not 1, so metres and cells differ.
    const CELL_M: f64 = 2.0;
    /// The raster of the fixtures: a 3 x 3 lattice of 8-cell blocks without offsets.
    const N: usize = 24;

    fn room(role: Role, cells: &[(u16, u16)], keys: &[KeyId]) -> Room {
        Room {
            ty: "chamber".to_owned(),
            role,
            cells: cells.iter().map(|&(row, col)| Cell { row, col }).collect(),
            shape: Shape::Rect,
            keys: keys.to_vec(),
            locked: false,
        }
    }

    fn conn(a: RoomId, b: RoomId, kind: Passage) -> Connection {
        Connection {
            a,
            b,
            kind,
            main: false,
        }
    }

    /// A synthetic interior on a 3 x 3 lattice (cell (2, 2) unused):
    ///
    /// ```text
    ///   (0,0) start r0 | (0,1)-(0,2) hall r1
    ///   (1,0) r2 key 1 | (1,1) r3   | (1,2) goal r4
    ///   (2,0) exit r5  | (2,1) leaf r6
    /// ```
    ///
    /// Connections, in order (door-1 .. door-8): 0-1, 0-2, 1-3 locked by key 1, 1-4, 2-3, 2-5,
    /// 3-4 a drop from the goal, 3-6.
    fn graph() -> RoomGraph {
        RoomGraph {
            rooms: vec![
                room(Role::Start, &[(0, 0)], &[]),
                room(Role::Path, &[(0, 1), (0, 2)], &[]),
                room(Role::Path, &[(1, 0)], &[1]),
                room(Role::Path, &[(1, 1)], &[]),
                room(Role::Goal, &[(1, 2)], &[]),
                room(Role::Exit, &[(2, 0)], &[]),
                room(Role::Leaf, &[(2, 1)], &[]),
            ],
            connections: vec![
                conn(0, 1, Passage::Open),
                conn(0, 2, Passage::Open),
                conn(1, 3, Passage::Locked(1)),
                conn(1, 4, Passage::Open),
                conn(2, 3, Passage::Open),
                conn(2, 5, Passage::Open),
                conn(3, 4, Passage::Drop { from: 4 }),
                conn(3, 6, Passage::Open),
            ],
            lattice: LatticeRef { rows: 3, cols: 3 },
        }
    }

    fn envelope() -> Grid<bool> {
        Grid::filled(N, N, true)
    }

    fn no_cover() -> Grid<bool> {
        Grid::filled(N, N, false)
    }

    fn space(g: &RoomGraph) -> SpaceOut {
        let mut rng = StageRng::for_key(0, "lattice");
        let lat = Lattice::new(N, N, 8, 0, Stagger::Rows, &mut rng).unwrap();
        let p = SpaceParams {
            wall: 1,
            door: 1,
            passage: 2,
            ragged_t: 0.0,
        };
        let flat = Grid::filled(N, N, 0.0);
        build_space(
            g,
            &lat,
            &envelope(),
            &p,
            &flat,
            &mut StageRng::for_key(1, "doors"),
        )
        .unwrap()
    }

    fn run(g: &RoomGraph, s: &SpaceOut, cover: &Grid<bool>) -> Analysis {
        analyse(g, s, cover, &envelope(), CELL_M, (N, N)).unwrap()
    }

    /// The Manhattan length in cells of the polyline through `points`.
    fn manhattan(points: &[(u32, u32)]) -> u32 {
        points
            .windows(2)
            .map(|w| w[0].0.abs_diff(w[1].0) + w[0].1.abs_diff(w[1].1))
            .sum()
    }

    /// The length in cells of a walk through rect rooms `rooms`, via their doors. Inside a rect
    /// room every centre-to-door and door-to-door walk is a Manhattan one.
    fn walk(g: &RoomGraph, s: &SpaceOut, rooms: &[usize]) -> u32 {
        let mut points = vec![s.centres[rooms[0]]];
        for w in rooms.windows(2) {
            let (lo, hi) = (w[0].min(w[1]), w[0].max(w[1]));
            let conn = g
                .connections
                .iter()
                .position(|c| usize::from(c.a) == lo && usize::from(c.b) == hi)
                .unwrap();
            points.push(s.doors.0[conn].centre);
        }
        points.push(s.centres[*rooms.last().unwrap()]);
        manhattan(&points)
    }

    fn whole(cells: u32) -> f64 {
        f64::from(cells) * CELL_M
    }

    // AC: CE-80.2
    #[test]
    fn a_clean_interior_has_no_findings_and_route_metres_are_whole_cells() {
        let g = graph();
        let s = space(&g);
        let a = run(&g, &s, &no_cover());
        assert_eq!(a.findings, Vec::<(Code, String)>::new());
        assert_eq!(a.route, [0, 1, 4]);
        // Each connection costs centre -> door -> centre, so the route passes every room centre.
        assert_eq!(
            a.route_points,
            [
                s.centres[0],
                s.doors.0[0].centre,
                s.centres[1],
                s.doors.0[3].centre,
                s.centres[4]
            ]
        );
        assert_eq!(a.route_points.first(), Some(&s.centres[0]));
        assert_eq!(a.route_points.last(), Some(&s.centres[4]));
        // In rect rooms every raster leg is a Manhattan one, so the polyline length is the
        // route's length, in whole cells.
        let cells = manhattan(&a.route_points);
        assert!(cells > 0);
        assert_eq!(a.critical_path_m, whole(cells));
        assert_eq!(a.start_to_exit_min_m, Some(whole(walk(&g, &s, &[0, 2, 5]))));
        let from_goal = [
            &[4, 3, 2, 5][..],
            &[4, 1, 0, 2, 5],
            &[4, 1, 3, 2, 5],
            &[4, 3, 1, 0, 2, 5],
        ]
        .iter()
        .map(|r| walk(&g, &s, r))
        .min()
        .unwrap();
        assert_eq!(a.goal_to_exit_min_m, Some(whole(from_goal)));
        assert_eq!(
            a.longest_sightline_m,
            whole(longest_sightline_cells(&s.walkable))
        );
        assert!(a.longest_sightline_m >= whole(14), "the hall's rows");
    }

    // AC: CE-80.2
    #[test]
    fn a_blank_room_is_uncarved() {
        let g = graph();
        let mut s = space(&g);
        for i in 0..s.rooms.len() {
            if s.rooms.as_slice()[i] == 7 {
                s.rooms.as_mut_slice()[i] = 0;
                s.walkable.as_mut_slice()[i] = false;
            }
        }
        let a = run(&g, &s, &no_cover());
        assert_eq!(
            a.findings,
            [(
                Code::UncarvedRoom,
                "uncarved_room: room-7 has no cell".to_owned()
            )]
        );
    }

    // AC: CE-80.2
    #[test]
    fn a_walled_off_door_disconnects_its_room() {
        let g = graph();
        let s = space(&g);
        let mut cover = no_cover();
        for &(r, c) in &s.doors.0[7].cells {
            cover.set(r as usize, c as usize, true);
        }
        let a = run(&g, &s, &cover);
        assert_eq!(
            a.findings,
            [(
                Code::Disconnected,
                "disconnected: room-7 cut off from room-1".to_owned()
            )]
        );
        // The rest stays measurable.
        assert_eq!(a.route, [0, 1, 4]);
        assert!(a.start_to_exit_min_m.is_some());
    }

    // AC: CE-80.2
    #[test]
    fn a_key_behind_its_own_lock_is_unreachable() {
        // Key 1 moves into room 3, and every way into room 3 is locked by it.
        let mut g = graph();
        g.rooms[2].keys.clear();
        g.rooms[3].keys = vec![1];
        for i in [4, 6] {
            g.connections[i].kind = Passage::Locked(1);
        }
        let s = space(&g);
        let a = run(&g, &s, &no_cover());
        assert_eq!(
            a.findings,
            [
                (
                    Code::UnreachableRoom,
                    "unreachable_room: room-4 needs key-1".to_owned()
                ),
                (
                    Code::UnreachableRoom,
                    "unreachable_room: room-7 needs key-1".to_owned()
                ),
            ]
        );
        assert_eq!(a.route, [0, 1, 4]);
        // With the goal locked too, there is no route at all.
        g.connections[3].kind = Passage::Locked(1);
        let a = run(&g, &s, &no_cover());
        assert_eq!(a.findings.len(), 3);
        assert_eq!(a.findings[1].1, "unreachable_room: room-5 needs key-1");
        assert_eq!(a.route, Vec::<RoomId>::new());
        assert_eq!(a.route_points, Vec::<(u32, u32)>::new());
        assert_eq!(a.critical_path_m, 0.0);
    }

    // AC: CE-80.2
    #[test]
    fn a_drop_into_a_dead_end_is_unsafe() {
        let mut g = graph();
        g.connections[5].kind = Passage::Drop { from: 2 };
        let s = space(&g);
        let a = run(&g, &s, &no_cover());
        assert_eq!(
            a.findings,
            [(
                Code::UnsafeDrop,
                "unsafe_drop: door-6 drops from room-3 into room-6, where the goal is not reachable with the keys surely held (the start room's and the drop room's)"
                    .to_owned()
            )]
        );
    }

    // AC: CE-80.2
    #[test]
    fn a_door_off_its_rooms_mismatches() {
        let g = graph();
        let mut s = space(&g);
        s.doors.0[0].rooms = (0, 2);
        let mut stray = s.doors.0[1].clone();
        stray.connection = 99;
        s.doors.0.push(stray);
        let a = run(&g, &s, &no_cover());
        assert_eq!(
            a.findings,
            [
                (
                    Code::DoorMismatch,
                    "door_mismatch: door-1 between room-1 and room-2 has 0 matching doors"
                        .to_owned()
                ),
                (
                    Code::DoorMismatch,
                    "door_mismatch: door-100 names no connection".to_owned()
                ),
            ]
        );
    }

    // AC: CE-80.2
    #[test]
    fn findings_are_all_collected_in_check_order() {
        let mut g = graph();
        g.connections[5].kind = Passage::Drop { from: 2 };
        let mut s = space(&g);
        s.doors.0[7].rooms = (3, 5);
        for i in 0..s.rooms.len() {
            if s.rooms.as_slice()[i] == 7 {
                s.rooms.as_mut_slice()[i] = 0;
                s.walkable.as_mut_slice()[i] = false;
            }
        }
        let a = run(&g, &s, &no_cover());
        let codes: Vec<Code> = a.findings.iter().map(|f| f.0).collect();
        assert_eq!(
            codes,
            [Code::UncarvedRoom, Code::UnsafeDrop, Code::DoorMismatch]
        );
        for (code, message) in &a.findings {
            assert!(message.starts_with(&format!("{}: ", code.as_str())));
        }
    }

    // AC: CE-80.2
    #[test]
    fn metrics_match_hand_counts() {
        let g = graph();
        let s = space(&g);
        let a = run(&g, &s, &no_cover());
        assert_eq!(a.rooms, 7.0);
        assert_eq!(a.connections, 8.0);
        // 8 connections - 7 rooms + 1 component: the loops 0-1-3-2 and 1-4-3.
        assert_eq!(a.cycles, 2.0);
        assert_eq!(a.locks, 1.0);
        assert_eq!(a.one_way, 1.0);
        // Room 2 alone holds room 5, room 3 alone holds room 6.
        assert_eq!(a.chokepoints, 2.0);
        // Rooms 5 and 6 have one connection each.
        assert_eq!(a.dead_end_pct, 200.0 / 7.0);
        // Room cells, per block of 8 with a 1-cell wall on the north and west lines, and on the
        // south and east lines where no room lies beyond (the frame, or the unused cell (2, 2)):
        // r0 7x7, r1 7x(7+7) without the inner wall, r2 7x7, r3 7x7, r4 6x6, r5 6x7, r6 6x6.
        let room_cells = 49 + 98 + 49 + 49 + 36 + 42 + 36;
        let walkable = room_cells + 8; // one cell per door
        assert_eq!(
            s.walkable.as_slice().iter().filter(|&&w| w).count(),
            walkable
        );
        assert_eq!(a.footprint_used_pct, 100.0 * 367.0 / 576.0);
        assert_eq!(a.hall_share_pct, 100.0 * 98.0 / 367.0);
        // Without an exit room the exit distances are absent.
        let mut no_exit = graph();
        no_exit.rooms[5].role = Role::Leaf;
        let b = run(&no_exit, &s, &no_cover());
        assert_eq!((b.start_to_exit_min_m, b.goal_to_exit_min_m), (None, None));
    }

    // AC: CE-80.2
    #[test]
    fn analysis_is_a_world_value() {
        let g = graph();
        let mut a = run(&g, &space(&g), &no_cover());
        a.findings
            .push((Code::UnsafeDrop, "unsafe_drop: door-1".to_owned()));
        assert_eq!(a.type_name(), "analysis");
        assert_eq!(Analysis::NAME, "analysis");
        let json = a.canonical_json().unwrap();
        assert_eq!(json["findings"][0][0], "unsafe_drop");
        assert_eq!(json["route"], serde_json::json!([0, 1, 4]));
        assert!(json["goal_to_exit_min_m"].is_number());
    }
}
