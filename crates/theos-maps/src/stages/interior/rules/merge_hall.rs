//! Rule `merge_hall` (spec §5.6). Mode `grow` extends a room whose program footprint exceeds
//! its cells to a block of that footprint over free cells; mode `merge` joins two adjacent
//! `path` rooms with an open connection between them into one hall. Neither changes a route:
//! growing only adds free cells, and merging rooms joined open keeps every walk.

use std::collections::BTreeSet;

use serde::Deserialize;

use crate::rng::StageRng;
use crate::rooms::{Cell, Connection, Passage, Role, RoomId};
use crate::stage::ParamError;
use crate::stages::interior::mission_core::{
    Match, MissionState, Rule, RuleParams, params_of, parse_params, sorted,
};

/// The rule `merge_hall`.
pub struct MergeHall;

/// What `merge_hall` does.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
enum Mode {
    /// Grow a room to its program footprint.
    #[default]
    Grow,
    /// Merge two adjacent open `path` rooms.
    Merge,
}

/// The parameters of `merge_hall`.
#[derive(Debug, Deserialize)]
struct Params {
    /// The mode; `grow` when left out.
    #[serde(default)]
    mode: Mode,
}

impl Rule for MergeHall {
    fn id(&self) -> &'static str {
        "merge_hall"
    }

    fn params(&self, table: &toml::Table) -> Result<Box<dyn RuleParams>, ParamError> {
        Ok(Box::new(parse_params::<Params>(self.id(), table)?))
    }

    /// `grow`: per room, every block of its footprint that holds all its cells and whose other
    /// cells are free. `merge`: every open connection between two `path` rooms without keys,
    /// neither `locked`. Weight 1.
    fn matches(&self, s: &MissionState, p: &dyn RuleParams) -> Vec<Match> {
        match params_of::<Params>(p).map(|p| p.mode) {
            Some(Mode::Grow) => grow_matches(s),
            Some(Mode::Merge) => merge_matches(s),
            None => Vec::new(),
        }
    }

    /// `grow`: the room's cells become the block. `merge`: the higher room joins the lower.
    fn apply(&self, s: &mut MissionState, m: &Match, p: &dyn RuleParams, _: &mut StageRng) {
        match (params_of::<Params>(p).map(|p| p.mode), &m.rooms[..]) {
            (Some(Mode::Grow), &[r]) => {
                if let Some(room) = s.graph.rooms.get_mut(usize::from(r)) {
                    room.cells.clone_from(&m.cells);
                }
            }
            (Some(Mode::Merge), &[keep, gone]) => merge(s, keep, gone),
            _ => {}
        }
    }

    fn makes_keys(&self, _: &dyn RuleParams, _: u32) -> u32 {
        0
    }
}

/// The cells of the `h` x `w` block whose top-left cell is `at`, sorted.
fn block(at: Cell, h: u16, w: u16) -> Vec<Cell> {
    (at.row..at.row + h)
        .flat_map(|row| (at.col..at.col + w).map(move |col| Cell { row, col }))
        .collect()
}

/// The growth matches of every room: the blocks of its footprint, either way round, inside the
/// lattice, holding all its cells, with every other cell free. A room already as large as its
/// footprint, or of a type the program does not name, has none.
fn grow_matches(s: &MissionState) -> Vec<Match> {
    let mut out = Vec::new();
    for (i, room) in s.graph.rooms.iter().enumerate() {
        let (Ok(id), Some((h, w)), Some(first), Some(last)) = (
            RoomId::try_from(i),
            s.program.footprint(&room.ty),
            room.cells.iter().map(|c| c.row).min(),
            room.cells.iter().map(|c| c.row).max(),
        ) else {
            continue;
        };
        let (h, w) = (u16::from(h), u16::from(w));
        if usize::from(h) * usize::from(w) <= room.cells.len() {
            continue;
        }
        let (Some(left), Some(right)) = (
            room.cells.iter().map(|c| c.col).min(),
            room.cells.iter().map(|c| c.col).max(),
        ) else {
            continue;
        };
        let mut blocks = BTreeSet::new();
        for (bh, bw) in [(h, w), (w, h)] {
            if bh > s.lattice.rows || bw > s.lattice.cols {
                continue;
            }
            let rows = (last + 1).saturating_sub(bh)..=first.min(s.lattice.rows.saturating_sub(bh));
            for row in rows {
                let cols =
                    (right + 1).saturating_sub(bw)..=left.min(s.lattice.cols.saturating_sub(bw));
                for col in cols {
                    let cells = block(Cell { row, col }, bh, bw);
                    let fits = cells
                        .iter()
                        .all(|c| room.cells.binary_search(c).is_ok() || s.is_free(*c));
                    if fits && room.cells.iter().all(|c| cells.contains(c)) {
                        blocks.insert(cells);
                    }
                }
            }
        }
        out.extend(blocks.into_iter().map(|cells| Match {
            cells,
            rooms: vec![id],
            weight: 1,
        }));
    }
    out
}

/// The merge matches: every open connection between two `path` rooms that hold no keys and are
/// not `locked`, as (a, b) with both rooms' cells.
fn merge_matches(s: &MissionState) -> Vec<Match> {
    let mergeable = |r: RoomId| {
        s.graph
            .rooms
            .get(usize::from(r))
            .is_some_and(|room| room.role == Role::Path && room.keys.is_empty() && !room.locked)
    };
    s.graph
        .connections
        .iter()
        .filter(|c| c.kind == Passage::Open && mergeable(c.a) && mergeable(c.b))
        .map(|c| Match {
            cells: s.cells_of(&[c.a, c.b]),
            rooms: vec![c.a, c.b],
            weight: 1,
        })
        .collect()
}

/// The order in which collapsing connections keep their kind: open before locked before drop,
/// then by key or drop room, so the choice is total.
fn rank(kind: Passage) -> (u8, u16) {
    match kind {
        Passage::Open => (0, 0),
        Passage::Locked(k) => (1, u16::from(k)),
        Passage::Drop { from } => (2, from),
    }
}

/// Merges room `b` into room `a < b`: `a` takes the union of the cells, `b`'s connections move
/// to `a` (the a–b connection goes; two that collapse into one keep the lowest [`rank`] and are
/// on the main cycle if either was), `b` is removed and every room number above `b` drops by
/// one.
fn merge(s: &mut MissionState, a: RoomId, b: RoomId) {
    if a >= b || usize::from(b) >= s.graph.rooms.len() {
        return;
    }
    let moved = s.graph.rooms.remove(usize::from(b));
    if let Some(room) = s.graph.rooms.get_mut(usize::from(a)) {
        room.cells.extend(moved.cells);
        room.cells = sorted(&room.cells);
    }
    let renumber = |r: RoomId| match r.cmp(&b) {
        std::cmp::Ordering::Less => r,
        std::cmp::Ordering::Equal => a,
        std::cmp::Ordering::Greater => r - 1,
    };
    let mut connections: Vec<Connection> = s
        .graph
        .connections
        .iter()
        .filter_map(|c| {
            let (one, two) = (renumber(c.a), renumber(c.b));
            let kind = match c.kind {
                Passage::Drop { from } => Passage::Drop {
                    from: renumber(from),
                },
                kind => kind,
            };
            (one != two).then_some(Connection {
                a: one.min(two),
                b: one.max(two),
                kind,
                main: c.main,
            })
        })
        .collect();
    connections.sort_by_key(|c| (c.a, c.b, rank(c.kind)));
    let mut out: Vec<Connection> = Vec::with_capacity(connections.len());
    for c in connections {
        match out.last_mut() {
            Some(last) if (last.a, last.b) == (c.a, c.b) => last.main |= c.main,
            _ => out.push(c),
        }
    }
    s.graph.connections = out;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rooms::Room;
    use crate::rooms::solve::solvable;
    use crate::stages::interior::rules::testkit::{
        cell, entry, lattice, program, program_room, room, run, state_on,
    };

    fn matches_of(s: &MissionState, params: &str) -> Vec<(Vec<Cell>, Vec<RoomId>)> {
        let p = MergeHall.params(&toml::from_str(params).unwrap()).unwrap();
        MergeHall
            .matches(s, p.as_ref())
            .into_iter()
            .map(|m| (m.cells, m.rooms))
            .collect()
    }

    fn cells(at: &[(u16, u16)]) -> Vec<Cell> {
        at.iter().map(|&(r, c)| cell(r, c)).collect()
    }

    /// The roles, keys and `locked` flags of the rooms.
    fn roles_and_keys(s: &MissionState) -> Vec<(Role, Vec<u8>, bool)> {
        s.graph
            .rooms
            .iter()
            .map(|r| (r.role, r.keys.clone(), r.locked))
            .collect()
    }

    /// On a 3x4 lattice: the start (0, 0), the goal (0, 1) and a `hall` path room (footprint
    /// 2x2) at (1, 0), each joined to the start. Synthetic fixture types.
    fn with_hall(footprint: (u8, u8)) -> MissionState {
        let hall = program_room("hall", Role::Path, footprint, false);
        let mut s = state_on(lattice(3, 4), &program(&[hall]));
        s.add_room(room("porch", Role::Start, cell(0, 0), false));
        s.add_room(room("altar", Role::Goal, cell(0, 1), false));
        s.add_room(room("hall", Role::Path, cell(1, 0), false));
        s.connect(0, 1, Passage::Open, true);
        s.connect(0, 2, Passage::Open, false);
        s
    }

    // AC: CE-80.2
    #[test]
    fn merge_hall_grows_to_the_footprint() {
        // The only 2x2 block around (1, 0) that leaves the start alone.
        let s = with_hall((2, 2));
        let grown = cells(&[(1, 0), (1, 1), (2, 0), (2, 1)]);
        assert_eq!(matches_of(&s, ""), [(grown.clone(), vec![2])]);
        assert_eq!(matches_of(&s, "mode = \"grow\""), matches_of(&s, ""));
        let mut t = s.clone();
        let r = run(&mut t, &[entry("merge_hall", 1, true, "")], 1);
        assert_eq!((r.applied, r.short), (1, 0));
        assert_eq!(t.graph.rooms[2].cells, grown);
        assert_eq!(t.graph.connections, s.graph.connections);
        assert_eq!(solvable(&t.graph), Ok(()));
        assert_eq!(
            matches_of(&t, ""),
            [],
            "a room at its footprint grows no more"
        );

        // A 1x2 footprint grows either way round: down or right, never up into the start.
        let s = with_hall((1, 2));
        assert_eq!(
            matches_of(&s, ""),
            [
                (cells(&[(1, 0), (1, 1)]), vec![2]),
                (cells(&[(1, 0), (2, 0)]), vec![2]),
            ]
        );

        // A 1x1 footprint, or a type the program does not name, has nothing to grow to.
        assert_eq!(matches_of(&with_hall((1, 1)), ""), []);
        let mut generic = with_hall((2, 2));
        generic.graph.rooms[2].ty = "path".to_owned();
        assert_eq!(matches_of(&generic, ""), []);
        assert!(
            MergeHall
                .params(&toml::from_str("mode = \"shrink\"").unwrap())
                .is_err()
        );
    }

    /// On a 3x3 lattice: a `hall` path room (footprint 2x2) at (1, 1) with the start at
    /// (0, 1), the goal at (1, 0) behind lock 1, and a `store` holding key 1 at `store`, joined
    /// to the start. Synthetic fixture types.
    fn hemmed_in(store: (u16, u16)) -> MissionState {
        let hall = program_room("hall", Role::Path, (2, 2), false);
        let mut s = state_on(lattice(3, 3), &program(&[hall]));
        s.add_room(room("porch", Role::Start, cell(0, 1), false));
        s.add_room(room("altar", Role::Goal, cell(1, 0), false));
        s.add_room(room("hall", Role::Path, cell(1, 1), false));
        let mut keyed = room("store", Role::Leaf, cell(store.0, store.1), false);
        keyed.keys.push(s.new_key());
        s.add_room(keyed);
        s.connect(0, 2, Passage::Open, true);
        s.connect(1, 2, Passage::Locked(1), true);
        let host = if store == (1, 2) { 2 } else { 0 };
        s.connect(host, 3, Passage::Open, false);
        s
    }

    /// A row of rooms on a 2x4 lattice: start (0, 0), path rooms 1 (0, 1) and 2 (0, 2), goal
    /// (0, 3); a `store` (1, 1)-(1, 2) below both path rooms, behind lock 1 from room 1 and with
    /// a drop into room 2; a leaf (1, 3) open to the store, with a drop into the goal. Key 1 is
    /// in the start. Synthetic fixture types.
    fn row_with_store() -> MissionState {
        let mut s = state_on(lattice(2, 4), &program(&[]));
        let mut start = room("porch", Role::Start, cell(0, 0), false);
        start.keys.push(s.new_key());
        s.add_room(start);
        s.add_room(room("gallery", Role::Path, cell(0, 1), false));
        s.add_room(room("gallery", Role::Path, cell(0, 2), false));
        s.add_room(room("altar", Role::Goal, cell(0, 3), false));
        s.add_room(Room {
            cells: cells(&[(1, 1), (1, 2)]),
            ..room("store", Role::Leaf, cell(1, 1), false)
        });
        s.add_room(room("cellar", Role::Leaf, cell(1, 3), false));
        s.connect(0, 1, Passage::Open, true);
        s.connect(1, 2, Passage::Open, true);
        s.connect(2, 3, Passage::Open, true);
        s.connect(1, 4, Passage::Locked(1), false);
        s.connect(2, 4, Passage::Drop { from: 4 }, false);
        s.connect(4, 5, Passage::Open, false);
        s.connect(3, 5, Passage::Drop { from: 5 }, false);
        s
    }

    // AC: CE-80.2
    #[test]
    fn merge_hall_merges_two_open_path_rooms() {
        let s = row_with_store();
        assert_eq!(solvable(&s.graph), Ok(()));
        assert_eq!(
            matches_of(&s, "mode = \"merge\""),
            [(cells(&[(0, 1), (0, 2)]), vec![1, 2])]
        );
        let mut t = s.clone();
        let r = run(
            &mut t,
            &[entry("merge_hall", 1, true, "mode = \"merge\"")],
            1,
        );
        assert_eq!((r.applied, r.short), (1, 0));
        let types: Vec<&str> = t.graph.rooms.iter().map(|r| r.ty.as_str()).collect();
        assert_eq!(types, ["porch", "gallery", "altar", "store", "cellar"]);
        assert_eq!(t.graph.rooms[1].cells, cells(&[(0, 1), (0, 2)]));
        assert_eq!(t.graph.rooms[1].role, Role::Path);
        let conns: Vec<(RoomId, RoomId, Passage, bool)> = t
            .graph
            .connections
            .iter()
            .map(|c| (c.a, c.b, c.kind, c.main))
            .collect();
        assert_eq!(
            conns,
            [
                (0, 1, Passage::Open, true),
                (1, 2, Passage::Open, true),
                (1, 3, Passage::Locked(1), false),
                (2, 4, Passage::Drop { from: 4 }, false),
                (3, 4, Passage::Open, false),
            ]
        );
        assert_eq!(solvable(&t.graph), Ok(()));
        assert_eq!(matches_of(&t, "mode = \"merge\""), []);
    }

    // AC: CE-80.2
    #[test]
    fn merge_hall_keeps_roles_and_keys() {
        // Review Focus 2. Hemmed in by the start, the goal and the key room, the hall has no
        // block of free cells: it never absorbs them.
        let s = hemmed_in((1, 2));
        assert_eq!(solvable(&s.graph), Ok(()));
        assert_eq!(matches_of(&s, ""), []);
        let mut t = s.clone();
        let r = run(&mut t, &[entry("merge_hall", 1, true, "")], 1);
        assert_eq!((r.applied, r.short), (0, 1));
        assert_eq!(t, s);

        // With the key room moved to (0, 2), only the block away from the three grows.
        let s = hemmed_in((0, 2));
        assert_eq!(solvable(&s.graph), Ok(()));
        let mut t = s.clone();
        let r = run(&mut t, &[entry("merge_hall", 1, true, "")], 1);
        assert_eq!((r.applied, r.short), (1, 0));
        assert_eq!(
            t.graph.rooms[2].cells,
            cells(&[(1, 1), (1, 2), (2, 1), (2, 2)])
        );
        assert_eq!(roles_and_keys(&t), roles_and_keys(&s));
        for i in [0, 1, 3] {
            assert_eq!(t.graph.rooms[i].cells, s.graph.rooms[i].cells, "room {i}");
        }

        // A merge never involves the start, the goal, a leaf, a keyed or a locked room.
        let mut s = row_with_store();
        assert_eq!(matches_of(&s, "mode = \"merge\"").len(), 1);
        s.graph.rooms[2].keys.push(s.next_key);
        assert_eq!(matches_of(&s, "mode = \"merge\""), []);
        let mut s = row_with_store();
        s.graph.rooms[1].locked = true;
        assert_eq!(matches_of(&s, "mode = \"merge\""), []);
        let mut s = row_with_store();
        s.graph.rooms[2].role = Role::Leaf;
        assert_eq!(matches_of(&s, "mode = \"merge\""), []);

        // The merged room keeps its role; every other room keeps its role and keys.
        let s = row_with_store();
        let mut t = s.clone();
        run(
            &mut t,
            &[entry("merge_hall", 1, true, "mode = \"merge\"")],
            1,
        );
        let mut expected = roles_and_keys(&s);
        expected.remove(2);
        assert_eq!(roles_and_keys(&t), expected);
        assert_eq!(
            MergeHall.makes_keys(MergeHall.params(&toml::Table::new()).unwrap().as_ref(), 9),
            0
        );
    }
}
