//! Rule `close_loop` (spec §5.6): adds an open connection between two adjacent, unconnected
//! rooms whose hop distance lies in `[min_dist, max_dist]`. Adding an open connection never
//! removes a route, so the graph stays solvable.

use serde::Deserialize;

use crate::rng::StageRng;
use crate::rooms::measure::hop_distance;
use crate::rooms::solve::fixed_point;
use crate::rooms::{Cell, Passage, RoomId};
use crate::stage::ParamError;
use crate::stages::interior::mission_core::{
    Match, MissionState, Rule, RuleParams, params_of, parse_params,
};

/// The rule `close_loop`.
pub struct CloseLoop;

/// The default of [`Params::min_dist`].
fn three() -> u32 {
    3
}

/// The default of [`Params::max_dist`].
fn seven() -> u32 {
    7
}

/// The parameters of `close_loop`.
#[derive(Debug, Deserialize)]
struct Params {
    /// The least hop distance of a pair.
    #[serde(default = "three")]
    min_dist: u32,
    /// The largest hop distance of a pair.
    #[serde(default = "seven")]
    max_dist: u32,
}

/// Whether rooms `a < b` are joined already.
fn connected(s: &MissionState, a: RoomId, b: RoomId) -> bool {
    s.graph
        .connections
        .binary_search_by(|c| (c.a, c.b).cmp(&(a, b)))
        .is_ok()
}

impl Rule for CloseLoop {
    fn id(&self) -> &'static str {
        "close_loop"
    }

    fn params(&self, table: &toml::Table) -> Result<Box<dyn RuleParams>, ParamError> {
        let p: Params = parse_params(self.id(), table)?;
        if p.min_dist > p.max_dist {
            return Err(ParamError::at(
                self.id(),
                "min_dist",
                format!("{} must be at most max_dist ({})", p.min_dist, p.max_dist),
            ));
        }
        Ok(Box::new(p))
    }

    /// Every pair of rooms `a < b` that are not joined, can have a door (ruling R1), lie at a hop
    /// distance in `[min_dist, max_dist]`, are both not `locked`, and are reached in the same
    /// round of the fixed point (an open connection across rounds would bypass a lock). The
    /// match holds both rooms' cells; weight 1, in (a, b) order.
    fn matches(&self, s: &MissionState, p: &dyn RuleParams) -> Vec<Match> {
        let Some(params) = params_of::<Params>(p) else {
            return Vec::new();
        };
        let graph = &s.graph;
        let round = fixed_point(graph, true).round;
        let mut out = Vec::new();
        for (i, ra) in graph.rooms.iter().enumerate() {
            let Ok(a) = RoomId::try_from(i) else {
                continue;
            };
            if ra.locked {
                continue;
            }
            let dist = hop_distance(graph, a);
            for (j, rb) in graph.rooms.iter().enumerate().skip(i + 1) {
                let Ok(b) = RoomId::try_from(j) else {
                    continue;
                };
                let in_range = dist
                    .get(j)
                    .copied()
                    .flatten()
                    .is_some_and(|d| (params.min_dist..=params.max_dist).contains(&d));
                if in_range
                    && !rb.locked
                    && round.get(i) == round.get(j)
                    && !connected(s, a, b)
                    && s.joinable(&ra.cells, &rb.cells)
                {
                    out.push(Match {
                        cells: cells_of(s, a, b),
                        rooms: vec![a, b],
                        weight: 1,
                    });
                }
            }
        }
        out
    }

    /// Joins the two rooms open, off the main cycle.
    fn apply(&self, s: &mut MissionState, m: &Match, _: &dyn RuleParams, _: &mut StageRng) {
        if let [a, b] = m.rooms[..] {
            s.connect(a, b, Passage::Open, false);
        }
    }

    fn makes_keys(&self, _: &dyn RuleParams, _: u32) -> u32 {
        0
    }
}

/// The sorted cells of rooms `a` and `b`.
fn cells_of(s: &MissionState, a: RoomId, b: RoomId) -> Vec<Cell> {
    let mut cells: Vec<Cell> = [a, b]
        .iter()
        .filter_map(|&r| s.graph.rooms.get(usize::from(r)))
        .flat_map(|r| r.cells.iter().copied())
        .collect();
    cells.sort_unstable();
    cells
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rooms::Role;
    use crate::rooms::lattice::Lattice;
    use crate::rooms::measure::cycles;
    use crate::rooms::solve::solvable;
    use crate::stages::interior::rules::testkit::{
        cell, entry, lattice, lattice_without_vertical_segments, program, room, run, state_on,
    };

    /// A chain of 1x1 rooms joined open in the order of `at`: the first is the start, the last
    /// the goal, the others path rooms (synthetic fixture types).
    fn chain(l: Lattice, at: &[(u16, u16)]) -> MissionState {
        let mut s = state_on(l, &program(&[]));
        let last = at.len() - 1;
        for (i, &(row, col)) in at.iter().enumerate() {
            let (ty, role) = match i {
                0 => ("porch", Role::Start),
                i if i == last => ("altar", Role::Goal),
                _ => ("gallery", Role::Path),
            };
            let id = s.add_room(room(ty, role, cell(row, col), false));
            if id > 0 {
                s.connect(id - 1, id, Passage::Open, true);
            }
        }
        s
    }

    /// A 2x4 snake: along row 0, then back along row 1. Its unjoined vertical pairs are rooms
    /// (0, 7) at distance 7, (1, 6) at 5 and (2, 5) at 3.
    fn snake() -> MissionState {
        chain(
            lattice(2, 4),
            &[
                (0, 0),
                (0, 1),
                (0, 2),
                (0, 3),
                (1, 3),
                (1, 2),
                (1, 1),
                (1, 0),
            ],
        )
    }

    fn pairs(s: &MissionState, params: &str) -> Vec<Vec<RoomId>> {
        let p = CloseLoop.params(&toml::from_str(params).unwrap()).unwrap();
        CloseLoop
            .matches(s, p.as_ref())
            .into_iter()
            .map(|m| {
                assert_eq!(m.cells, cells_of(s, m.rooms[0], m.rooms[1]));
                m.rooms
            })
            .collect()
    }

    // AC: CE-80.2
    #[test]
    fn close_loop_respects_distance_and_count() {
        let s = snake();
        assert_eq!(pairs(&s, ""), [vec![0, 7], vec![1, 6], vec![2, 5]]);
        assert_eq!(pairs(&s, "min_dist = 4\nmax_dist = 6"), [vec![1, 6]]);
        assert_eq!(pairs(&s, "min_dist = 6"), [vec![0, 7]]);
        assert_eq!(
            pairs(&s, "min_dist = 8\nmax_dist = 9"),
            Vec::<Vec<RoomId>>::new()
        );

        for seed in 1..=8 {
            let mut s = snake();
            assert_eq!(cycles(&s.graph), 0);
            let r = run(&mut s, &[entry("close_loop", 2, true, "")], seed);
            assert_eq!((r.applied, r.short, r.per_entry[0].undone), (2, 0, 0));
            assert_eq!(cycles(&s.graph), 2, "count = 2 adds exactly 2 cycles");
            assert_eq!(s.graph.rooms.len(), 8);
            let added: Vec<_> = s.graph.connections.iter().filter(|c| !c.main).collect();
            assert_eq!(added.len(), 2);
            assert!(added.iter().all(|c| c.kind == Passage::Open));
            assert_eq!(solvable(&s.graph), Ok(()));
        }

        // Locked rooms are never joined.
        let mut s = snake();
        s.graph.rooms[6].locked = true;
        assert_eq!(pairs(&s, ""), [vec![0, 7], vec![2, 5]]);

        // Rooms behind a lock (a later round of the fixed point) are never joined to rooms
        // before it: that would bypass the lock.
        let mut s = snake();
        s.graph.connections[3].kind = Passage::Locked(1);
        s.graph.rooms[3].keys.push(1);
        s.next_key = 2;
        assert_eq!(pairs(&s, ""), Vec::<Vec<RoomId>>::new());

        let err = CloseLoop
            .params(&toml::from_str("min_dist = 5\nmax_dist = 4").unwrap())
            .err()
            .unwrap();
        assert_eq!(err.parameter.as_deref(), Some("min_dist"));
        assert_eq!(CloseLoop.makes_keys(pairs_params().as_ref(), 9), 0);
    }

    fn pairs_params() -> Box<dyn RuleParams> {
        CloseLoop.params(&toml::Table::new()).unwrap()
    }

    // AC: CE-80.2
    #[test]
    fn close_loop_needs_shared_segment() {
        // Start (0, 0) and goal (1, 0) touch, at hop distance 3 through column 1, but their
        // shared wall is 3 raster cells long: no door fits, so they are never joined.
        let at = [(0, 0), (0, 1), (1, 1), (1, 0)];
        let s = chain(lattice_without_vertical_segments(), &at);
        assert!(!s.joinable(&[cell(0, 0)], &[cell(1, 0)]));
        assert!(s.joinable(&[cell(0, 1)], &[cell(1, 1)]));
        assert_eq!(pairs(&s, ""), Vec::<Vec<RoomId>>::new());
        let mut short = s.clone();
        let r = run(&mut short, &[entry("close_loop", 1, true, "")], 1);
        assert_eq!((r.applied, r.short), (0, 1));
        assert_eq!(short, s);

        // The same rooms on a regular lattice are joined.
        let regular = chain(lattice(2, 3), &at);
        assert_eq!(pairs(&regular, ""), [vec![0, 3]]);

        // Multi-cell rooms join when at least one pair of their cells has a segment.
        assert!(s.joinable(&[cell(0, 0), cell(0, 1)], &[cell(1, 0), cell(1, 1)]));
        assert!(!s.joinable(&[cell(0, 0), cell(0, 2)], &[cell(1, 0), cell(1, 2)]));
    }
}
