//! The lock-and-key solver on a [`RoomGraph`] (§5.9, checks 3 and 4) and the critical route.
//!
//! Keys are held as a 16-bit mask (bit `k - 1` is key `k`) and are picked up when a room is
//! entered. A locked passage is two-way once its key is held; a drop is walkable from its `from`
//! room to the other end only.

use std::cmp::{Ordering, Reverse};
use std::collections::{BTreeMap, BinaryHeap};

use crate::rooms::graph::{KeyId, Passage, RoomGraph, RoomId};
use crate::rooms::measure::hop_distance;

/// What a flood from the start reaches.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Reach {
    /// Per room: whether it was reached.
    pub rooms: Vec<bool>,
    /// The keys held at the end, as a mask (bit `k - 1` is key `k`).
    pub keys: u16,
    /// Per room: the flood (from 0) that first reached it.
    pub round: Vec<Option<u16>>,
}

/// Why a graph is not solvable.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Unsolvable {
    /// A room is not reached by the fixed point.
    UnreachableRoom {
        /// The lowest room that is not reached.
        room: RoomId,
        /// The keys of the locks between reached and unreached rooms that are not held.
        missing: Vec<KeyId>,
    },
    /// A drop can leave the party in a place from which the goal is out of reach.
    UnsafeDrop {
        /// The index of the first unsafe drop in [`RoomGraph::connections`].
        connection: usize,
    },
}

/// The mask bit of a key; keys above 16 cannot occur and count as never held.
fn bit(k: KeyId) -> u16 {
    debug_assert!((1..=16).contains(&k), "key {k} is outside 1..=16");
    if (1..=16).contains(&k) {
        1 << (k - 1)
    } else {
        0
    }
}

/// The keys found in a room, as a mask.
fn room_keys(g: &RoomGraph, room: usize) -> u16 {
    g.rooms
        .get(room)
        .map_or(0, |r| r.keys.iter().fold(0, |m, &k| m | bit(k)))
}

/// Per room, the connections that touch it as `(other room, connection index)`.
fn adjacency(g: &RoomGraph) -> Vec<Vec<(usize, usize)>> {
    let mut adj = vec![Vec::new(); g.rooms.len()];
    for (i, c) in g.connections.iter().enumerate() {
        let (a, b) = (usize::from(c.a), usize::from(c.b));
        if a < adj.len() && b < adj.len() {
            adj[a].push((b, i));
            adj[b].push((a, i));
        }
    }
    adj
}

/// Whether connection `conn` can be walked from `from` with the keys in `mask`.
fn passable(g: &RoomGraph, conn: usize, from: usize, mask: u16, drops: bool) -> bool {
    match g.connections[conn].kind {
        Passage::Open => true,
        Passage::Locked(k) => mask & bit(k) != 0,
        Passage::Drop { from: f } => drops && usize::from(f) == from,
    }
}

/// The start and goal rooms, if the graph has them.
fn start_goal(g: &RoomGraph) -> Option<(usize, usize)> {
    use crate::rooms::graph::Role;
    let find = |role| g.rooms.iter().position(|r| r.role == role);
    Some((find(Role::Start)?, find(Role::Goal)?))
}

/// Floods from `start` with the keys `keys0` (plus the start room's) until no new key turns up.
fn expand(
    g: &RoomGraph,
    adj: &[Vec<(usize, usize)>],
    start: usize,
    keys0: u16,
    drops: bool,
) -> Reach {
    let n = g.rooms.len();
    let mut reach = Reach {
        rooms: vec![false; n],
        keys: keys0 | room_keys(g, start),
        round: vec![None; n],
    };
    if start >= n {
        return reach;
    }
    let mut round: u16 = 0;
    loop {
        // One flood from `start` with the keys held now.
        let mut seen = vec![false; n];
        seen[start] = true;
        let mut stack = vec![start];
        while let Some(room) = stack.pop() {
            for &(next, conn) in &adj[room] {
                if !seen[next] && passable(g, conn, room, reach.keys, drops) {
                    seen[next] = true;
                    stack.push(next);
                }
            }
        }
        let mut keys = reach.keys;
        for (room, &s) in seen.iter().enumerate() {
            if s {
                if !reach.rooms[room] {
                    reach.rooms[room] = true;
                    reach.round[room] = Some(round);
                }
                keys |= room_keys(g, room);
            }
        }
        if keys == reach.keys {
            return reach;
        }
        reach.keys = keys;
        round = round.saturating_add(1);
    }
}

/// The fixed point of §5.9 check 3: flood, collect keys, repeat. With `drops == false` the drops
/// are removed. A graph without a start room reaches nothing.
#[must_use]
pub fn fixed_point(g: &RoomGraph, drops: bool) -> Reach {
    let adj = adjacency(g);
    match start_goal(g) {
        Some((start, _)) => expand(g, &adj, start, 0, drops),
        None => Reach {
            rooms: vec![false; g.rooms.len()],
            keys: 0,
            round: vec![None; g.rooms.len()],
        },
    }
}

/// The drops (as connection indices) after which the goal can be out of reach (§5.9 check 4).
///
/// From the drop's target room, with the keys surely held (those reachable from the start with
/// every drop removed) and every key found on the way, the goal must be reachable.
#[must_use]
pub fn unsafe_drops(g: &RoomGraph) -> Vec<usize> {
    let adj = adjacency(g);
    let Some((start, goal)) = start_goal(g) else {
        return Vec::new();
    };
    let surely = expand(g, &adj, start, 0, false).keys;
    g.connections
        .iter()
        .enumerate()
        .filter_map(|(i, c)| {
            let Passage::Drop { from } = c.kind else {
                return None;
            };
            debug_assert!(
                from == c.a || from == c.b,
                "a drop starts at one of its ends"
            );
            let target = if from == c.a { c.b } else { c.a };
            let after = expand(g, &adj, usize::from(target), surely, true);
            (!after.rooms[goal]).then_some(i)
        })
        .collect()
}

/// Whether every room is reachable and every drop is safe.
///
/// # Errors
/// [`Unsolvable::UnreachableRoom`] for the lowest room the fixed point does not reach, else
/// [`Unsolvable::UnsafeDrop`] for the first unsafe drop.
pub fn solvable(g: &RoomGraph) -> Result<(), Unsolvable> {
    let reach = fixed_point(g, true);
    if let Some(room) = reach.rooms.iter().position(|&r| !r) {
        let mut missing: Vec<KeyId> = g
            .connections
            .iter()
            .filter_map(|c| {
                let Passage::Locked(k) = c.kind else {
                    return None;
                };
                let (a, b) = (usize::from(c.a), usize::from(c.b));
                let across = reach.rooms.get(a) != reach.rooms.get(b);
                (across && reach.keys & bit(k) == 0).then_some(k)
            })
            .collect();
        missing.sort_unstable();
        missing.dedup();
        return Err(Unsolvable::UnreachableRoom {
            room: RoomId::try_from(room).unwrap_or(RoomId::MAX),
            missing,
        });
    }
    match unsafe_drops(g).first() {
        Some(&connection) => Err(Unsolvable::UnsafeDrop { connection }),
        None => Ok(()),
    }
}

/// An `f64` ordered by [`f64::total_cmp`].
#[derive(Debug, Clone, Copy, PartialEq)]
struct Length(f64);

impl Eq for Length {}

impl PartialOrd for Length {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Length {
    fn cmp(&self, other: &Self) -> Ordering {
        self.0.total_cmp(&other.0)
    }
}

/// The shortest walk from start to goal that respects locks and drops, with its length.
///
/// `len(connection, entered_from)` is the cost of crossing a connection from the room
/// `entered_from`. The search is a Dijkstra over (room, key mask), ordered by (length, room id,
/// mask). The route lists every room visited, revisits included. `None` if the goal cannot be
/// reached.
pub fn critical_route(
    g: &RoomGraph,
    len: &mut dyn FnMut(usize, RoomId) -> f64,
) -> Option<(Vec<RoomId>, f64)> {
    type State = (usize, u16);
    let (start, goal) = start_goal(g)?;
    let adj = adjacency(g);
    let first: State = (start, room_keys(g, start));
    let mut best: BTreeMap<State, f64> = BTreeMap::new();
    let mut parent: BTreeMap<State, State> = BTreeMap::new();
    let mut heap = BinaryHeap::new();
    best.insert(first, 0.0);
    heap.push(Reverse((Length(0.0), first.0, first.1)));
    while let Some(Reverse((Length(dist), room, mask))) = heap.pop() {
        let state = (room, mask);
        if best
            .get(&state)
            .is_some_and(|&d| d.total_cmp(&dist).is_lt())
        {
            continue;
        }
        if room == goal {
            let mut route = vec![RoomId::try_from(room).unwrap_or(RoomId::MAX)];
            let mut at = state;
            while let Some(&before) = parent.get(&at) {
                route.push(RoomId::try_from(before.0).unwrap_or(RoomId::MAX));
                at = before;
            }
            route.reverse();
            return Some((route, dist));
        }
        let from_id = RoomId::try_from(room).unwrap_or(RoomId::MAX);
        for &(next, conn) in &adj[room] {
            if !passable(g, conn, room, mask, true) {
                continue;
            }
            let state_next = (next, mask | room_keys(g, next));
            let cost = dist + len(conn, from_id);
            if best
                .get(&state_next)
                .is_none_or(|&d| cost.total_cmp(&d).is_lt())
            {
                best.insert(state_next, cost);
                parent.insert(state_next, state);
                heap.push(Reverse((Length(cost), next, state_next.1)));
            }
        }
    }
    None
}

/// Per room, in room order: (the round the fixed point with every drop reaches it, its hop
/// distance from the start, its id). Comparing two tuples gives the "later" order of the
/// `shortcut` rule. Rooms never reached sort last.
#[must_use]
pub fn later_order(g: &RoomGraph) -> Vec<(u16, u32, RoomId)> {
    let reach = fixed_point(g, true);
    let hops =
        start_goal(g).map(|(s, _)| hop_distance(g, RoomId::try_from(s).unwrap_or(RoomId::MAX)));
    (0..g.rooms.len())
        .map(|i| {
            let hop = hops.as_ref().and_then(|h| h[i]);
            (
                reach.round[i].unwrap_or(u16::MAX),
                hop.unwrap_or(u32::MAX),
                RoomId::try_from(i).unwrap_or(RoomId::MAX),
            )
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rooms::graph::{Cell, Connection, LatticeRef, Role, Room, Shape};

    type C = (RoomId, RoomId, Passage);

    /// Room 0 is the start, the last room the goal; `keys` is (room, key).
    fn graph(n: usize, keys: &[(usize, KeyId)], conns: &[C]) -> RoomGraph {
        let rooms = (0..n)
            .map(|i| Room {
                ty: "hall".to_owned(),
                role: if i == 0 {
                    Role::Start
                } else if i == n - 1 {
                    Role::Goal
                } else {
                    Role::Path
                },
                cells: vec![Cell { row: 0, col: 0 }],
                shape: Shape::Rect,
                keys: keys
                    .iter()
                    .filter(|&&(r, _)| r == i)
                    .map(|&(_, k)| k)
                    .collect(),
                locked: false,
            })
            .collect();
        RoomGraph {
            rooms,
            connections: conns
                .iter()
                .map(|&(a, b, kind)| Connection {
                    a,
                    b,
                    kind,
                    main: false,
                })
                .collect(),
            lattice: LatticeRef { rows: 1, cols: 1 },
        }
    }

    // AC: CE-80.2
    #[test]
    fn key_in_start_room() {
        let g = graph(2, &[(0, 1)], &[(0, 1, Passage::Locked(1))]);
        assert_eq!(solvable(&g), Ok(()));
        let r = fixed_point(&g, true);
        assert_eq!(r.keys, 1);
        assert_eq!(r.rooms, [true, true]);
        assert_eq!(r.round, [Some(0), Some(0)]);
    }

    // AC: CE-80.2
    #[test]
    fn lock_whose_key_is_behind_another_lock_solves_in_order() {
        let g = graph(
            4,
            &[(1, 1), (2, 2)],
            &[
                (0, 1, Passage::Open),
                (1, 2, Passage::Locked(1)),
                (2, 3, Passage::Locked(2)),
            ],
        );
        assert_eq!(solvable(&g), Ok(()));
        let r = fixed_point(&g, true);
        assert_eq!(r.keys, 0b11);
        assert_eq!(r.round, [Some(0), Some(0), Some(1), Some(2)]);
    }

    // AC: CE-80.2
    #[test]
    fn key_behind_its_own_lock_is_unreachable() {
        let g = graph(2, &[(1, 1)], &[(0, 1, Passage::Locked(1))]);
        assert_eq!(
            solvable(&g),
            Err(Unsolvable::UnreachableRoom {
                room: 1,
                missing: vec![1]
            })
        );
        let r = fixed_point(&g, true);
        assert_eq!(r.rooms, [true, false]);
        assert_eq!(r.round, [Some(0), None]);
    }

    // AC: CE-80.2
    #[test]
    fn drop_into_goal_is_safe() {
        let g = graph(
            3,
            &[],
            &[(0, 1, Passage::Open), (1, 2, Passage::Drop { from: 1 })],
        );
        assert_eq!(unsafe_drops(&g), Vec::<usize>::new());
        assert_eq!(solvable(&g), Ok(()));
    }

    // AC: CE-80.2
    #[test]
    fn drop_from_start_is_refused_when_it_skips_a_needed_key() {
        // The goal (room 3) lies behind lock 2 from room 1; key 2 lies in room 2, which only
        // its own drop from the start reaches. Falling into room 1 skips that key.
        let g = graph(
            4,
            &[(2, 2)],
            &[
                (0, 1, Passage::Drop { from: 0 }),
                (0, 2, Passage::Drop { from: 0 }),
                (1, 3, Passage::Locked(2)),
            ],
        );
        assert_eq!(unsafe_drops(&g), [0, 1]);
        assert_eq!(solvable(&g), Err(Unsolvable::UnsafeDrop { connection: 0 }));
    }

    // AC: CE-80.2
    #[test]
    fn two_drops_in_a_row() {
        let g = graph(
            3,
            &[],
            &[
                (0, 1, Passage::Drop { from: 0 }),
                (1, 2, Passage::Drop { from: 1 }),
            ],
        );
        assert_eq!(unsafe_drops(&g), Vec::<usize>::new());
        assert_eq!(solvable(&g), Ok(()));
        // A drop is never walkable backwards.
        let back = graph(2, &[], &[(0, 1, Passage::Drop { from: 1 })]);
        assert_eq!(fixed_point(&back, true).rooms, [true, false]);
        let down = graph(2, &[], &[(0, 1, Passage::Drop { from: 0 })]);
        assert_eq!(fixed_point(&down, false).rooms, [true, false]);
    }

    fn two_key_graph() -> RoomGraph {
        graph(
            5,
            &[(1, 1), (2, 2)],
            &[
                (0, 1, Passage::Open),
                (0, 2, Passage::Open),
                (0, 3, Passage::Locked(1)),
                (3, 4, Passage::Locked(2)),
            ],
        )
    }

    // AC: CE-80.2
    #[test]
    fn critical_route_fetches_both_keys() {
        let g = two_key_graph();
        // Fetching key 1 first and key 2 first tie at 6; the state (room 1, both keys) pops before
        // (room 2, both keys), so the route that ends its fetching in room 1 wins.
        let (route, len) = critical_route(&g, &mut |_, _| 1.0).expect("a route");
        assert_eq!(route, [0, 2, 0, 1, 0, 3, 4]);
        assert_eq!(len, 6.0);
        let lens = [1.0, 2.0, 10.0, 4.0];
        let (route, len) = critical_route(&g, &mut |c, _| lens[c]).expect("a route");
        // Equal totals again; here the cheaper detour (room 1) comes first.
        assert_eq!(route, [0, 1, 0, 2, 0, 3, 4]);
        assert_eq!(len, 20.0);
    }

    // AC: CE-80.2
    #[test]
    fn critical_route_breaks_ties_by_room_id() {
        let g = graph(
            4,
            &[],
            &[
                (0, 1, Passage::Open),
                (0, 2, Passage::Open),
                (1, 3, Passage::Open),
                (2, 3, Passage::Open),
            ],
        );
        let (route, len) = critical_route(&g, &mut |_, _| 1.0).expect("a route");
        assert_eq!(route, [0, 1, 3]);
        assert_eq!(len, 2.0);
    }

    // AC: CE-80.2
    #[test]
    fn route_revisits_a_room() {
        let g = graph(
            3,
            &[(1, 1)],
            &[(0, 1, Passage::Open), (0, 2, Passage::Locked(1))],
        );
        let (route, len) = critical_route(&g, &mut |_, _| 1.0).expect("a route");
        assert_eq!(route, [0, 1, 0, 2]);
        assert_eq!(len, 3.0);
    }

    // AC: CE-80.2
    #[test]
    fn route_is_none_when_the_goal_cannot_be_reached() {
        let g = graph(2, &[(1, 1)], &[(0, 1, Passage::Locked(1))]);
        assert_eq!(critical_route(&g, &mut |_, _| 1.0), None);
    }

    fn chain(n: u8) -> RoomGraph {
        let keys: Vec<(usize, KeyId)> = (0..n).map(|i| (usize::from(i), i + 1)).collect();
        let conns: Vec<C> = (0..n)
            .map(|i| (RoomId::from(i), RoomId::from(i) + 1, Passage::Locked(i + 1)))
            .collect();
        graph(usize::from(n) + 1, &keys, &conns)
    }

    // AC: CE-80.2
    #[test]
    fn sixteen_keys_solve() {
        let g = chain(16);
        assert_eq!(solvable(&g), Ok(()));
        let r = fixed_point(&g, true);
        assert_eq!(r.keys, u16::MAX);
        assert_eq!(r.round[16], Some(15));
        let (route, len) = critical_route(&g, &mut |_, _| 1.0).expect("a route");
        assert_eq!(route.len(), 17);
        assert_eq!(len, 16.0);
    }

    // AC: CE-80.2
    #[test]
    fn later_order_ranks_rounds_then_hops_then_ids() {
        let g = graph(
            4,
            &[(1, 1)],
            &[
                (0, 1, Passage::Open),
                (0, 2, Passage::Open),
                (2, 3, Passage::Locked(1)),
            ],
        );
        assert_eq!(
            later_order(&g),
            [(0, 0, 0), (0, 1, 1), (0, 1, 2), (1, 2, 3)]
        );
    }
}
