//! Rule `insert_cycle` (spec §5.6): lays a detour of new `path` rooms around an open
//! connection u–v, along the border of a 2x2, 2x3 (either way round) or 3x3 block of cells. The
//! old connection stays, so the detour closes exactly one new cycle. With `lock = true` the
//! detour's connection into the far room is locked and its key lies in the detour's first room,
//! on the near side: the key is reached before its lock.

use std::collections::BTreeSet;

use serde::Deserialize;

use crate::rng::StageRng;
use crate::rooms::solve::later_order;
use crate::rooms::{Cell, Passage, Role, Room, RoomId};
use crate::stage::ParamError;
use crate::stages::interior::mission_core::{
    Match, MissionState, Rule, RuleParams, near_far, params_of, parse_params,
};

/// The rule `insert_cycle`.
pub struct InsertCycle;

/// The block shapes (rows, cols) a detour runs around, in the order they are tried.
const SHAPES: [(u16, u16); 4] = [(2, 2), (2, 3), (3, 2), (3, 3)];

/// The parameters of `insert_cycle`.
#[derive(Debug, Deserialize)]
struct Params {
    /// Whether the detour's far connection is locked.
    #[serde(default)]
    lock: bool,
}

/// The border of the `h` x `w` block whose top-left cell is `at`, clockwise from `at`. The
/// caller keeps the block inside the lattice.
fn ring(at: Cell, h: u16, w: u16) -> Vec<Cell> {
    let (r0, c0) = (at.row, at.col);
    let (r1, c1) = (r0 + h - 1, c0 + w - 1);
    let mut out = Vec::new();
    out.extend((c0..=c1).map(|col| Cell { row: r0, col }));
    out.extend((r0 + 1..=r1).map(|row| Cell { row, col: c1 }));
    out.extend((c0..c1).rev().map(|col| Cell { row: r1, col }));
    out.extend((r0 + 1..r1).rev().map(|row| Cell { row, col: c0 }));
    out
}

/// The top-left cells of every `h` x `w` block inside the lattice that holds one of `cells`,
/// in (row, col) order.
fn blocks_around(s: &MissionState, cells: &[Cell], h: u16, w: u16) -> BTreeSet<Cell> {
    let (rows, cols) = (s.lattice.rows, s.lattice.cols);
    let mut out = BTreeSet::new();
    if rows < h || cols < w {
        return out;
    }
    for c in cells {
        let rows_at = c.row.saturating_sub(h - 1)..=c.row.min(rows - h);
        for row in rows_at {
            let cols_at = c.col.saturating_sub(w - 1)..=c.col.min(cols - w);
            out.extend(cols_at.map(|col| Cell { row, col }));
        }
    }
    out
}

/// The detours on `ring` from the cells `near` of one room to the cells `far` of another, each in
/// order from `near`: where a cell of each follow each other on the ring, the rest of the ring the other way
/// round, when its cells are free and every step, the two ends included, can have a door.
fn ring_detours(s: &MissionState, ring: &[Cell], near: &[Cell], far: &[Cell]) -> Vec<Vec<Cell>> {
    let n = ring.len();
    let in_room = |room: &[Cell], cell: Cell| room.binary_search(&cell).is_ok();
    let mut out = Vec::new();
    for i in 0..n {
        let j = (i + 1) % n;
        let (from, to, path): (Cell, Cell, Vec<Cell>) =
            if in_room(near, ring[i]) && in_room(far, ring[j]) {
                let back = (1..n - 1).map(|t| ring[(i + n - t) % n]).collect();
                (ring[i], ring[j], back)
            } else if in_room(far, ring[i]) && in_room(near, ring[j]) {
                let ahead = (1..n - 1).map(|t| ring[(j + t) % n]).collect();
                (ring[j], ring[i], ahead)
            } else {
                continue;
            };
        let steps = std::iter::once(from)
            .chain(path.iter().copied())
            .chain(std::iter::once(to))
            .collect::<Vec<_>>();
        let ok = path.iter().all(|&cell| s.is_free(cell))
            && steps
                .windows(2)
                .all(|step| s.joinable(&step[..1], &step[1..]));
        if ok {
            out.push(path);
        }
    }
    out
}

/// The detours from room `u` to room `v` around the first shape of [`SHAPES`] that has any,
/// each in order from u, each cell set once.
fn detours(s: &MissionState, u: RoomId, v: RoomId) -> Vec<Vec<Cell>> {
    let rooms = &s.graph.rooms;
    let (Some(ru), Some(rv)) = (rooms.get(usize::from(u)), rooms.get(usize::from(v))) else {
        return Vec::new();
    };
    for (h, w) in SHAPES {
        let mut found: Vec<Vec<Cell>> = Vec::new();
        let mut seen: BTreeSet<Vec<Cell>> = BTreeSet::new();
        for at in blocks_around(s, &ru.cells, h, w) {
            for path in ring_detours(s, &ring(at, h, w), &ru.cells, &rv.cells) {
                if seen.insert(sorted(&path)) {
                    found.push(path);
                }
            }
        }
        if !found.is_empty() {
            return found;
        }
    }
    Vec::new()
}

/// `cells` sorted: the cells of a match. Shared by the rules that place a path of rooms.
pub(super) fn sorted(cells: &[Cell]) -> Vec<Cell> {
    let mut out = cells.to_vec();
    out.sort_unstable();
    out
}

impl Rule for InsertCycle {
    fn id(&self) -> &'static str {
        "insert_cycle"
    }

    fn params(&self, table: &toml::Table) -> Result<Box<dyn RuleParams>, ParamError> {
        Ok(Box::new(parse_params::<Params>(self.id(), table)?))
    }

    /// Per open connection, its rooms as (near, far) and every detour of the first shape that
    /// fits; the match holds the detour's cells, weight 1. Locked connections and drops are
    /// never offered.
    fn matches(&self, s: &MissionState, _: &dyn RuleParams) -> Vec<Match> {
        let order = later_order(&s.graph);
        let mut out = Vec::new();
        for c in s
            .graph
            .connections
            .iter()
            .filter(|c| c.kind == Passage::Open)
        {
            let (u, v) = near_far(&order, c.a, c.b);
            out.extend(detours(s, u, v).into_iter().map(|path| Match {
                cells: sorted(&path),
                rooms: vec![u, v],
                weight: 1,
            }));
        }
        out
    }

    /// Places the detour as 1x1 open `path` rooms of the program, joined open in order from the
    /// near room; the last joins the far room open, or locked by a new key that lies in the
    /// detour's first room.
    fn apply(&self, s: &mut MissionState, m: &Match, p: &dyn RuleParams, _: &mut StageRng) {
        let (Some(params), &[near, far]) = (params_of::<Params>(p), m.rooms.as_slice()) else {
            return;
        };
        let Some(path) = detours(s, near, far)
            .into_iter()
            .find(|path| sorted(path) == m.cells)
        else {
            return;
        };
        let key = params.lock.then(|| s.new_key());
        let mut prev = near;
        let mut first = None;
        for &cell in &path {
            let room = Room {
                cells: vec![cell],
                ..s.program.take_with_lock(Role::Path, false)
            };
            let id = s.add_room(room);
            s.connect(prev, id, Passage::Open, false);
            first.get_or_insert(id);
            prev = id;
        }
        let last = key.map_or(Passage::Open, Passage::Locked);
        s.connect(prev, far, last, false);
        if let (Some(key), Some(first)) = (key, first)
            && let Some(room) = s.graph.rooms.get_mut(usize::from(first))
        {
            room.keys.push(key);
        }
    }

    fn makes_keys(&self, p: &dyn RuleParams, count_max: u32) -> u32 {
        match params_of::<Params>(p) {
            Some(p) if p.lock => count_max,
            _ => 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rooms::measure::cycles;
    use crate::rooms::solve::{fixed_point, solvable};
    use crate::stages::interior::rules::testkit::{
        cell, entry, lattice, program, room, run, state_on,
    };

    /// Start (0, 0) and goal (0, 1) joined by `kind` on a `rows` x `cols` lattice; with
    /// `blocker`, a leaf at (1, 1) joined open to the goal.
    fn pair(rows: usize, cols: usize, kind: Passage, blocker: bool) -> MissionState {
        let mut s = state_on(lattice(rows, cols), &program(&[]));
        let mut start = room("porch", Role::Start, cell(0, 0), false);
        if let Passage::Locked(k) = kind {
            start.keys.push(k);
            s.next_key = k + 1;
        }
        let a = s.add_room(start);
        let b = s.add_room(room("altar", Role::Goal, cell(0, 1), false));
        s.connect(a, b, kind, true);
        if blocker {
            let c = s.add_room(room("shed", Role::Leaf, cell(1, 1), false));
            s.connect(b, c, Passage::Open, false);
        }
        s
    }

    fn parsed(text: &str) -> Box<dyn RuleParams> {
        InsertCycle.params(&toml::from_str(text).unwrap()).unwrap()
    }

    fn sorted_matches(s: &MissionState) -> Vec<(Vec<Cell>, Vec<RoomId>)> {
        let mut all: Vec<_> = InsertCycle
            .matches(s, parsed("").as_ref())
            .into_iter()
            .map(|m| (m.cells, m.rooms))
            .collect();
        all.sort();
        all
    }

    fn valid_and_solvable(s: &MissionState) {
        assert_eq!(s.graph.validate(&|c| s.lattice.is_enabled(c)), Ok(()));
        assert_eq!(solvable(&s.graph), Ok(()));
    }

    const TWO_ROUTES: &str = "side = \"south\"\ncycle = \"two_routes\"";
    const LOCK_KEY: &str = "side = \"south\"\ncycle = \"lock_key\"";

    // AC: CE-80.2
    #[test]
    fn insert_cycle_adds_one_cycle() {
        // A free 3x3 lattice: the 2x2 block is tried first and fits.
        let free = pair(3, 3, Passage::Open, false);
        assert_eq!(
            sorted_matches(&free),
            [(vec![cell(1, 0), cell(1, 1)], vec![0, 1])]
        );

        // The leaf at (1, 1) blocks every 2x2, 2x3 and 3x2 detour of start-goal; the 3x3
        // border leaves the centre out and fits. The goal-leaf connection still has a 2x2.
        let blocked = pair(3, 3, Passage::Open, true);
        let ring = vec![
            cell(0, 2),
            cell(1, 0),
            cell(1, 2),
            cell(2, 0),
            cell(2, 1),
            cell(2, 2),
        ];
        assert_eq!(
            sorted_matches(&blocked),
            [
                (ring, vec![0, 1]),
                (vec![cell(0, 2), cell(1, 2)], vec![1, 2])
            ]
        );
        let mut one = blocked.clone();
        let report = run(&mut one, &[entry("insert_cycle", 2, false, "")], 3);
        assert_eq!((report.applied, report.per_entry[0].undone), (2, 0));
        valid_and_solvable(&one);
        assert_eq!(cycles(&one.graph), 2);

        for seed in 1..=8 {
            let mut s = state_on(lattice(6, 6), &program(&[]));
            run(&mut s, &[entry("main_cycle", 1, true, TWO_ROUTES)], seed);
            let before = s.graph.clone();
            let r = run(&mut s, &[entry("insert_cycle", 1, true, "")], seed);
            assert_eq!((r.applied, r.short, r.per_entry[0].undone), (1, 0, 0));
            valid_and_solvable(&s);
            assert_eq!(cycles(&s.graph), cycles(&before) + 1);
            assert!(
                before
                    .connections
                    .iter()
                    .all(|c| s.graph.connections.contains(c))
            );
            let new_rooms = &s.graph.rooms[before.rooms.len()..];
            assert!([2, 4, 6].contains(&new_rooms.len()));
            assert!(new_rooms.iter().all(|r| r.role == Role::Path && !r.locked));
            for c in s
                .graph
                .connections
                .iter()
                .filter(|c| !before.connections.contains(c))
            {
                assert_eq!((c.kind, c.main), (Passage::Open, false));
                let (low, high) = (
                    &s.graph.rooms[usize::from(c.a)],
                    &s.graph.rooms[usize::from(c.b)],
                );
                assert!(
                    s.joinable(&low.cells, &high.cells),
                    "every new connection has a door"
                );
            }
        }
    }

    // AC: CE-80.2
    #[test]
    fn insert_cycle_with_lock_stays_solvable() {
        assert_eq!(InsertCycle.makes_keys(parsed("lock = true").as_ref(), 3), 3);
        assert_eq!(InsertCycle.makes_keys(parsed("").as_ref(), 3), 0);
        for seed in 1..=12 {
            let mut s = state_on(lattice(6, 6), &program(&[]));
            let entries = [
                entry("main_cycle", 1, true, LOCK_KEY),
                entry("insert_cycle", 2, true, "lock = true"),
            ];
            let r = run(&mut s, &entries, seed);
            assert_eq!((r.applied, r.short), (3, 0));
            assert!(r.per_entry.iter().all(|e| e.undone == 0));
            valid_and_solvable(&s);
            assert_eq!(s.next_key, 4);
            let order = later_order(&s.graph);
            let locks: Vec<(usize, RoomId, RoomId, u8)> = s
                .graph
                .connections
                .iter()
                .enumerate()
                .filter_map(|(i, c)| match c.kind {
                    Passage::Locked(k) if !c.main => Some((i, c.a, c.b, k)),
                    _ => None,
                })
                .collect();
            assert_eq!(locks.len(), 2);
            for (i, a, b, key) in locks {
                let (near, far) = near_far(&order, a, b);
                assert_eq!(s.graph.rooms[usize::from(near)].role, Role::Path);
                let holder = s
                    .graph
                    .rooms
                    .iter()
                    .position(|r| r.keys.contains(&key))
                    .unwrap();
                assert_ne!(holder, usize::from(far));
                assert_eq!(s.graph.rooms[holder].role, Role::Path);
                let mut without = s.graph.clone();
                without.connections.remove(i);
                assert!(
                    fixed_point(&without, true).rooms[holder],
                    "key {key} before its lock"
                );
            }
        }
    }

    // AC: CE-80.2
    #[test]
    fn insert_cycle_on_a_locked_connection_does_not_match() {
        let locked = pair(3, 3, Passage::Locked(1), false);
        assert_eq!(sorted_matches(&locked), []);
        let mut after = locked.clone();
        let report = run(&mut after, &[entry("insert_cycle", 1, true, "")], 1);
        assert_eq!((report.applied, report.short), (0, 1));
        assert_eq!(after, locked);
        assert_eq!(
            sorted_matches(&pair(3, 3, Passage::Drop { from: 1 }, false)),
            []
        );

        // On the main cycle of `lock_key`, only open connections are offered.
        for seed in 1..=8 {
            let mut s = state_on(lattice(6, 6), &program(&[]));
            run(&mut s, &[entry("main_cycle", 1, true, LOCK_KEY)], seed);
            let all = InsertCycle.matches(&s, parsed("").as_ref());
            assert_ne!(all.len(), 0);
            for m in all {
                let pair = (m.rooms[0].min(m.rooms[1]), m.rooms[0].max(m.rooms[1]));
                let joined = s
                    .graph
                    .connections
                    .iter()
                    .find(|c| (c.a, c.b) == pair)
                    .unwrap();
                assert_eq!(joined.kind, Passage::Open);
            }
        }
    }

    #[test]
    fn unknown_keys_are_refused() {
        let err = InsertCycle
            .params(&toml::from_str("locked = true").unwrap())
            .err()
            .unwrap();
        assert_eq!(err.parameter.as_deref(), Some("locked"));
    }
}
