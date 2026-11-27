//! Rule `extend_arc` (spec §5.6): inserts new `path` rooms into a connection u–v along free
//! cells. The new rooms are open towards the near room u and the last one takes the old
//! connection's kind towards the far room v, so a lock keeps its key on the near side.

use serde::Deserialize;

use crate::rng::StageRng;
use crate::rooms::solve::later_order;
use crate::rooms::{Cell, Passage, Role, Room, RoomId};
use crate::stage::ParamError;
use crate::stages::interior::mission_core::{
    Match, MissionState, Rule, RuleParams, near_far, parse_params,
};
use crate::stages::interior::rules::insert_cycle::sorted;

/// The rule `extend_arc`.
pub struct ExtendArc;

/// The parameters of `extend_arc`: none.
#[derive(Debug, Deserialize)]
struct Params {}

/// The bridges of free cells from room cells `u` to room cells `v`, each in order from u, every
/// step able to have a door (ruling R1): one cell joinable to both rooms; when there is none,
/// two neighbouring cells, the first joinable to u, the second to v. On a square lattice two
/// 1x1 neighbours have no common neighbour cell, so their bridges are always two cells long.
fn bridges(s: &MissionState, u: &[Cell], v: &[Cell]) -> Vec<Vec<Cell>> {
    let near = s.joinable_free(u);
    let ones: Vec<Vec<Cell>> = near
        .iter()
        .filter(|&&w| s.joinable(&[w], v))
        .map(|&w| vec![w])
        .collect();
    if !ones.is_empty() {
        return ones;
    }
    near.iter()
        .flat_map(|&w1| {
            s.joinable_free(&[w1])
                .into_iter()
                .filter(|&w2| s.joinable(&[w2], v))
                .map(move |w2| vec![w1, w2])
        })
        .collect()
}

/// The bridges from room `u` to room `v` (see [`bridges`]).
fn room_bridges(s: &MissionState, u: RoomId, v: RoomId) -> Vec<Vec<Cell>> {
    let rooms = &s.graph.rooms;
    match (rooms.get(usize::from(u)), rooms.get(usize::from(v))) {
        (Some(ru), Some(rv)) => bridges(s, &ru.cells, &rv.cells),
        _ => Vec::new(),
    }
}

impl Rule for ExtendArc {
    fn id(&self) -> &'static str {
        "extend_arc"
    }

    fn params(&self, table: &toml::Table) -> Result<Box<dyn RuleParams>, ParamError> {
        Ok(Box::new(parse_params::<Params>(self.id(), table)?))
    }

    /// Per open or locked connection (never a drop), its rooms as (near, far) and every bridge
    /// between them; the match holds the bridge's cells, weight 1.
    fn matches(&self, s: &MissionState, _: &dyn RuleParams) -> Vec<Match> {
        let order = later_order(&s.graph);
        let mut out = Vec::new();
        for c in &s.graph.connections {
            if matches!(c.kind, Passage::Drop { .. }) {
                continue;
            }
            let (u, v) = near_far(&order, c.a, c.b);
            out.extend(room_bridges(s, u, v).into_iter().map(|path| Match {
                cells: sorted(&path),
                rooms: vec![u, v],
                weight: 1,
            }));
        }
        out
    }

    /// Replaces the connection by the bridge's 1x1 open `path` rooms of the program: open from
    /// the near room on, the old kind into the far room, every new connection on the main cycle
    /// when the old one was.
    fn apply(&self, s: &mut MissionState, m: &Match, _: &dyn RuleParams, _: &mut StageRng) {
        let [u, v] = m.rooms[..] else {
            return;
        };
        let Some(path) = room_bridges(s, u, v)
            .into_iter()
            .find(|path| sorted(path) == m.cells)
        else {
            return;
        };
        let pair = (u.min(v), u.max(v));
        let Some(at) = s.graph.connections.iter().position(|c| (c.a, c.b) == pair) else {
            return;
        };
        let old = s.graph.connections.remove(at);
        let mut prev = u;
        for &cell in &path {
            let room = Room {
                cells: vec![cell],
                ..s.program.take_with_lock(Role::Path, false)
            };
            let id = s.add_room(room);
            s.connect(prev, id, Passage::Open, old.main);
            prev = id;
        }
        s.connect(prev, v, old.kind, old.main);
    }

    fn makes_keys(&self, _: &dyn RuleParams, _: u32) -> u32 {
        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rooms::solve::solvable;
    use crate::stages::interior::rules::testkit::{
        cell, entry, lattice, program, room, run, state_on,
    };

    /// Start (0, 0) with key 1 and a goal on `goal` joined by `kind` on a 3x3 lattice.
    fn pair(goal: &[Cell], kind: Passage) -> MissionState {
        let mut s = state_on(lattice(3, 3), &program(&[]));
        let mut start = room("porch", Role::Start, cell(0, 0), false);
        start.keys.push(1);
        s.next_key = 2;
        let a = s.add_room(start);
        let mut g = room("altar", Role::Goal, goal[0], false);
        g.cells = goal.to_vec();
        let b = s.add_room(g);
        s.connect(a, b, kind, true);
        s
    }

    fn offered(s: &MissionState) -> Vec<(Vec<Cell>, Vec<RoomId>)> {
        let p = ExtendArc.params(&toml::Table::new()).unwrap();
        ExtendArc
            .matches(s, p.as_ref())
            .into_iter()
            .map(|m| (m.cells, m.rooms))
            .collect()
    }

    fn kinds(s: &MissionState) -> Vec<(RoomId, RoomId, Passage, bool)> {
        s.graph
            .connections
            .iter()
            .map(|c| (c.a, c.b, c.kind, c.main))
            .collect()
    }

    // AC: CE-80.2
    #[test]
    fn extend_arc_keeps_the_far_kind() {
        // 1x1 rooms side by side have no common neighbour cell, so the bridge is two cells.
        let s = pair(&[cell(0, 1)], Passage::Locked(1));
        assert_eq!(offered(&s), [(vec![cell(1, 0), cell(1, 1)], vec![0, 1])]);
        let mut s2 = s.clone();
        let r = run(&mut s2, &[entry("extend_arc", 1, true, "")], 1);
        assert_eq!((r.applied, r.per_entry[0].undone), (1, 0));
        assert_eq!(solvable(&s2.graph), Ok(()));
        assert_eq!(
            kinds(&s2),
            [
                (0, 2, Passage::Open, true),
                (1, 3, Passage::Locked(1), true),
                (2, 3, Passage::Open, true),
            ]
        );
        assert_eq!(s2.graph.rooms[2].cells, [cell(1, 0)]);
        assert_eq!(s2.graph.rooms[3].cells, [cell(1, 1)]);
        assert!(
            s2.graph.rooms[2..]
                .iter()
                .all(|r| r.role == Role::Path && !r.locked)
        );

        // A two-cell goal shares the neighbour (1, 0) with the start: one new room, open on the
        // near side and open (the old kind) on the far side, off the main cycle as before.
        let mut s = pair(&[cell(0, 1), cell(1, 1)], Passage::Open);
        s.graph.connections[0].main = false;
        assert_eq!(offered(&s), [(vec![cell(1, 0)], vec![0, 1])]);
        run(&mut s, &[entry("extend_arc", 1, true, "")], 1);
        assert_eq!(
            kinds(&s),
            [(0, 2, Passage::Open, false), (1, 2, Passage::Open, false)]
        );

        // The goal behind the lock is the far room even when its id is lower.
        let mut s = state_on(lattice(3, 3), &program(&[]));
        let goal = s.add_room(room("altar", Role::Goal, cell(0, 1), false));
        let mut start = room("porch", Role::Start, cell(0, 0), false);
        start.keys.push(1);
        let start = s.add_room(start);
        s.next_key = 2;
        s.connect(start, goal, Passage::Locked(1), true);
        assert_eq!(offered(&s), [(vec![cell(1, 0), cell(1, 1)], vec![1, 0])]);
        run(&mut s, &[entry("extend_arc", 1, true, "")], 1);
        assert_eq!(
            kinds(&s),
            [
                (0, 3, Passage::Locked(1), true),
                (1, 2, Passage::Open, true),
                (2, 3, Passage::Open, true),
            ]
        );

        // Drops are never extended.
        assert_eq!(offered(&pair(&[cell(0, 1)], Passage::Drop { from: 1 })), []);
    }
}
