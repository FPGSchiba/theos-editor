//! Rule `shortcut` (spec §5.6): joins two adjacent, unconnected rooms whose shortest route is at
//! least `min_route` hops by a one-way drop from the later room to the earlier, in the order of
//! [`later_order`]. A drop toward an earlier room only returns the party to ground it has
//! walked; the guard's drop check (§5.9) confirms the goal stays reachable.

use serde::Deserialize;

use crate::rng::StageRng;
use crate::rooms::measure::hop_distance;
use crate::rooms::solve::later_order;
use crate::rooms::{Passage, Role, RoomId};
use crate::stage::ParamError;
use crate::stages::interior::mission_core::{
    Match, MissionState, Rule, RuleParams, near_far, params_of, parse_params,
};

/// The rule `shortcut`.
pub struct Shortcut;

/// The default of [`Params::min_route`].
fn four() -> u32 {
    4
}

/// The parameters of `shortcut`.
#[derive(Debug, Deserialize)]
struct Params {
    /// The least hop distance between the two rooms before the drop.
    #[serde(default = "four")]
    min_route: u32,
}

impl Rule for Shortcut {
    fn id(&self) -> &'static str {
        "shortcut"
    }

    fn params(&self, table: &toml::Table) -> Result<Box<dyn RuleParams>, ParamError> {
        Ok(Box::new(parse_params::<Params>(self.id(), table)?))
    }

    /// Every pair of rooms `a < b`, neither the goal, that are not joined, can have a door
    /// (ruling R1) and lie at least `min_route` hops apart (every passage counted, both ways).
    /// The match names the later room first, then the earlier; its cells are both rooms'
    /// cells; weight 1, in (a, b) order.
    fn matches(&self, s: &MissionState, p: &dyn RuleParams) -> Vec<Match> {
        let Some(params) = params_of::<Params>(p) else {
            return Vec::new();
        };
        let graph = &s.graph;
        let order = later_order(graph);
        let mut out = Vec::new();
        for (i, ra) in graph.rooms.iter().enumerate() {
            let Ok(a) = RoomId::try_from(i) else {
                continue;
            };
            if ra.role == Role::Goal {
                continue;
            }
            let dist = hop_distance(graph, a);
            for (j, rb) in graph.rooms.iter().enumerate().skip(i + 1) {
                let Ok(b) = RoomId::try_from(j) else {
                    continue;
                };
                let far_enough = dist
                    .get(j)
                    .copied()
                    .flatten()
                    .is_some_and(|d| d >= params.min_route);
                if far_enough
                    && rb.role != Role::Goal
                    && !s.connected(a, b)
                    && s.joinable(&ra.cells, &rb.cells)
                {
                    let (earlier, later) = near_far(&order, a, b);
                    out.push(Match {
                        cells: s.cells_of(&[a, b]),
                        rooms: vec![later, earlier],
                        weight: 1,
                    });
                }
            }
        }
        out
    }

    /// Joins the two rooms by a drop from the later room, off the main cycle.
    fn apply(&self, s: &mut MissionState, m: &Match, _: &dyn RuleParams, _: &mut StageRng) {
        if let [later, earlier] = m.rooms[..] {
            s.connect(later, earlier, Passage::Drop { from: later }, false);
        }
    }

    fn makes_keys(&self, _: &dyn RuleParams, _: u32) -> u32 {
        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rooms::lattice::Lattice;
    use crate::rooms::solve::{solvable, unsafe_drops};
    use crate::stages::interior::rules::testkit::{
        cell, entry, lattice, program, room, run, state_on,
    };

    /// Rooms of 1x1 on `l` (the first is the start, the room at `goal` the goal, the rest path
    /// rooms) joined by `conns`; `keys` is (room, key). Synthetic fixture types.
    fn graph(
        l: Lattice,
        at: &[(u16, u16)],
        goal: usize,
        conns: &[(RoomId, RoomId, Passage)],
        keys: &[(usize, u8)],
    ) -> MissionState {
        let mut s = state_on(l, &program(&[]));
        for (i, &(row, col)) in at.iter().enumerate() {
            let (ty, role) = match i {
                0 => ("porch", Role::Start),
                i if i == goal => ("altar", Role::Goal),
                _ => ("gallery", Role::Path),
            };
            s.add_room(room(ty, role, cell(row, col), false));
        }
        for &(a, b, kind) in conns {
            s.connect(a, b, kind, false);
        }
        for &(r, k) in keys {
            s.graph.rooms[r].keys.push(k);
            s.next_key = s.next_key.max(k + 1);
        }
        s
    }

    fn pairs(s: &MissionState, params: &str) -> Vec<Vec<RoomId>> {
        let p = Shortcut.params(&toml::from_str(params).unwrap()).unwrap();
        Shortcut
            .matches(s, p.as_ref())
            .into_iter()
            .map(|m| {
                assert_eq!(m.cells, s.cells_of(&m.rooms));
                m.rooms
            })
            .collect()
    }

    /// A 2x4 snake: along row 0, then back along row 1; room 7 is the goal. Its unjoined
    /// vertical pairs are rooms (0, 7), (1, 6) at distance 5 and (2, 5) at 3.
    fn snake() -> MissionState {
        let at = [
            (0, 0),
            (0, 1),
            (0, 2),
            (0, 3),
            (1, 3),
            (1, 2),
            (1, 1),
            (1, 0),
        ];
        let conns: Vec<_> = (0..7).map(|i| (i, i + 1, Passage::Open)).collect();
        graph(lattice(2, 4), &at, 7, &conns, &[])
    }

    // AC: CE-80.2
    #[test]
    fn shortcut_drops_from_later_to_earlier() {
        // The goal (room 7) is never part of a pair; `min_route` filters by hop distance.
        let s = snake();
        assert_eq!(pairs(&s, ""), [vec![6, 1]]);
        assert_eq!(pairs(&s, "min_route = 3"), [vec![6, 1], vec![5, 2]]);
        assert_eq!(pairs(&s, "min_route = 6"), Vec::<Vec<RoomId>>::new());
        for seed in 1..=4 {
            let mut s = snake();
            let r = run(&mut s, &[entry("shortcut", 1, true, "")], seed);
            assert_eq!((r.applied, r.short), (1, 0));
            let c = s.graph.connections.iter().find(|c| (c.a, c.b) == (1, 6));
            assert_eq!(
                c.map(|c| (c.kind, c.main)),
                Some((Passage::Drop { from: 6 }, false))
            );
            assert_eq!(solvable(&s.graph), Ok(()));
        }

        // "Later" is the fixed point's round before the hop distance: a 2x3 ring where room 1
        // (one hop from the start) lies behind a lock whose key is in room 5, while room 4 (two
        // hops) is reached in the first round. The drop starts in room 1.
        let at = [(0, 0), (0, 1), (0, 2), (1, 2), (1, 1), (1, 0)];
        let conns = [
            (0, 1, Passage::Locked(1)),
            (1, 2, Passage::Locked(1)),
            (2, 3, Passage::Open),
            (3, 4, Passage::Open),
            (4, 5, Passage::Open),
            (0, 5, Passage::Open),
        ];
        let ring = graph(lattice(2, 3), &at, 3, &conns, &[(5, 1)]);
        assert_eq!(solvable(&ring.graph), Ok(()));
        assert_eq!(pairs(&ring, "min_route = 3"), [vec![1, 4]]);
        let mut done = ring.clone();
        let r = run(&mut done, &[entry("shortcut", 1, true, "min_route = 3")], 1);
        assert_eq!((r.applied, r.short), (1, 0));
        assert!(done.graph.connections.contains(&crate::rooms::Connection {
            a: 1,
            b: 4,
            kind: Passage::Drop { from: 1 },
            main: false,
        }));
        assert_eq!(
            Shortcut.makes_keys(Shortcut.params(&toml::Table::new()).unwrap().as_ref(), 9),
            0
        );
    }

    // AC: CE-80.2
    #[test]
    fn shortcut_between_rooms_behind_different_locks_stays_safe() {
        // Spec §8 item 2. Branch A: start -(lock 1)- 1 - 2, key 2 in room 2; branch B:
        // start -(lock 2)- 3 - 4 - goal 5; key 1 in the start. Rooms 1 (behind lock 1) and 4
        // (behind lock 2) touch and are 3 hops apart.
        let at = [(0, 0), (0, 1), (0, 2), (1, 0), (1, 1), (1, 2)];
        let conns = [
            (0, 1, Passage::Locked(1)),
            (1, 2, Passage::Open),
            (0, 3, Passage::Locked(2)),
            (3, 4, Passage::Open),
            (4, 5, Passage::Open),
        ];
        let s = graph(lattice(2, 3), &at, 5, &conns, &[(0, 1), (2, 2)]);
        assert_eq!(solvable(&s.graph), Ok(()));
        let offered = pairs(&s, "min_route = 3");
        assert!(offered.contains(&vec![4, 1]), "{offered:?}");
        for seed in 1..=8 {
            let mut t = s.clone();
            run(
                &mut t,
                &[entry("shortcut", 3, false, "min_route = 1")],
                seed,
            );
            assert!(
                t.graph
                    .connections
                    .iter()
                    .any(|c| matches!(c.kind, Passage::Drop { .. })),
                "seed {seed}: a drop was applied"
            );
            assert_eq!(unsafe_drops(&t.graph), Vec::<usize>::new(), "seed {seed}");
            assert_eq!(solvable(&t.graph), Ok(()), "seed {seed}");
        }
    }
}
