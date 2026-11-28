//! Measures of a [`RoomGraph`]: cycles, chokepoints, dead ends and hop distances.

use crate::rooms::graph::{RoomGraph, RoomId};

/// Per room, the rooms joined to it (every passage counts, both directions).
fn adjacency(g: &RoomGraph) -> Vec<Vec<usize>> {
    let mut adj = vec![Vec::new(); g.rooms.len()];
    for c in &g.connections {
        let (a, b) = (usize::from(c.a), usize::from(c.b));
        if a < adj.len() && b < adj.len() {
            adj[a].push(b);
            adj[b].push(a);
        }
    }
    adj
}

/// The hop distance from `from` to every room over all connections; `None` if unreachable.
#[must_use]
pub fn hop_distance(g: &RoomGraph, from: RoomId) -> Vec<Option<u32>> {
    let adj = adjacency(g);
    let mut dist = vec![None; adj.len()];
    let start = usize::from(from);
    if start >= adj.len() {
        return dist;
    }
    dist[start] = Some(0);
    let mut queue = std::collections::VecDeque::from([start]);
    while let Some(room) = queue.pop_front() {
        let next_hop = dist[room].map_or(0, |d| d + 1);
        for &next in &adj[room] {
            if dist[next].is_none() {
                dist[next] = Some(next_hop);
                queue.push_back(next);
            }
        }
    }
    dist
}

/// The number of independent cycles: connections - rooms + connected components.
#[must_use]
pub fn cycles(g: &RoomGraph) -> usize {
    let mut seen = vec![false; g.rooms.len()];
    let mut components = 0;
    for room in 0..g.rooms.len() {
        if !seen[room] {
            components += 1;
            let reached = hop_distance(g, RoomId::try_from(room).unwrap_or(RoomId::MAX));
            for (r, d) in reached.iter().enumerate() {
                seen[r] |= d.is_some();
            }
        }
    }
    (g.connections.len() + components).saturating_sub(g.rooms.len())
}

/// The rooms whose removal splits their component, in room order (iterative Tarjan).
#[must_use]
pub fn articulation_points(g: &RoomGraph) -> Vec<RoomId> {
    let adj = adjacency(g);
    let n = adj.len();
    let mut disc = vec![0_usize; n]; // 0 = not visited; discovery times start at 1
    let mut low = vec![0_usize; n];
    let mut cut = vec![false; n];
    let mut clock = 0;
    for root in 0..n {
        if disc[root] != 0 {
            continue;
        }
        clock += 1;
        disc[root] = clock;
        low[root] = clock;
        let mut root_children = 0;
        // (room, parent, next neighbour index)
        let mut stack = vec![(root, usize::MAX, 0_usize)];
        while let Some(top) = stack.last_mut() {
            let (room, parent) = (top.0, top.1);
            if let Some(&next) = adj[room].get(top.2) {
                top.2 += 1;
                if disc[next] == 0 {
                    clock += 1;
                    disc[next] = clock;
                    low[next] = clock;
                    if room == root {
                        root_children += 1;
                    }
                    stack.push((next, room, 0));
                } else if next != parent {
                    low[room] = low[room].min(disc[next]);
                }
            } else {
                stack.pop();
                if parent != usize::MAX {
                    low[parent] = low[parent].min(low[room]);
                    if parent != root && low[room] >= disc[parent] {
                        cut[parent] = true;
                    }
                }
            }
        }
        if root_children > 1 {
            cut[root] = true;
        }
    }
    cut.iter()
        .enumerate()
        .filter(|&(_, &c)| c)
        .map(|(i, _)| RoomId::try_from(i).unwrap_or(RoomId::MAX))
        .collect()
}

/// The share of rooms with exactly one connection, in percent; 0 for a graph without rooms.
#[must_use]
pub fn dead_end_pct(g: &RoomGraph) -> f64 {
    if g.rooms.is_empty() {
        return 0.0;
    }
    let dead = adjacency(g).iter().filter(|a| a.len() == 1).count();
    let count = |n: usize| f64::from(u32::try_from(n).unwrap_or(u32::MAX));
    count(dead) * 100.0 / count(g.rooms.len())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rooms::graph::{Cell, Connection, LatticeRef, Passage, Role, Room, Shape};

    fn graph(n: usize, edges: &[(RoomId, RoomId)]) -> RoomGraph {
        RoomGraph {
            rooms: (0..n)
                .map(|i| Room {
                    ty: "hall".to_owned(),
                    role: if i == 0 { Role::Start } else { Role::Path },
                    cells: vec![Cell { row: 0, col: 0 }],
                    shape: Shape::Rect,
                    keys: Vec::new(),
                    locked: false,
                })
                .collect(),
            connections: edges
                .iter()
                .map(|&(a, b)| Connection {
                    a,
                    b,
                    kind: Passage::Open,
                    main: false,
                })
                .collect(),
            lattice: LatticeRef { rows: 1, cols: 1 },
        }
    }

    fn figure_eight() -> RoomGraph {
        graph(5, &[(0, 1), (0, 2), (0, 3), (0, 4), (1, 2), (3, 4)])
    }

    // AC: CE-80.2
    #[test]
    fn cycles_and_articulation_points() {
        let g = figure_eight();
        assert_eq!(cycles(&g), 2);
        assert_eq!(articulation_points(&g), [0]);
    }

    // AC: CE-80.2
    #[test]
    fn a_path_has_inner_articulation_points_and_no_cycles() {
        let g = graph(4, &[(0, 1), (1, 2), (2, 3)]);
        assert_eq!(cycles(&g), 0);
        assert_eq!(articulation_points(&g), [1, 2]);
        let two = graph(4, &[(0, 1), (2, 3)]);
        assert_eq!(cycles(&two), 0);
        assert_eq!(articulation_points(&two), Vec::<RoomId>::new());
    }

    // AC: CE-80.2
    #[test]
    fn dead_end_pct_of_a_star() {
        let star = graph(5, &[(0, 1), (0, 2), (0, 3), (0, 4)]);
        assert_eq!(dead_end_pct(&star), 80.0);
        assert_eq!(dead_end_pct(&figure_eight()), 0.0);
        assert_eq!(dead_end_pct(&graph(0, &[])), 0.0);
    }

    // AC: CE-80.2
    #[test]
    fn hop_distance_ignores_passage_kinds() {
        let mut g = graph(4, &[(0, 1), (1, 2)]);
        g.connections[1].kind = Passage::Locked(1);
        assert_eq!(hop_distance(&g, 0), [Some(0), Some(1), Some(2), None]);
        assert_eq!(hop_distance(&g, 9), [None, None, None, None]);
    }

    /// A long chain must not overflow the stack.
    #[test]
    fn articulation_points_of_a_long_chain() {
        let edges: Vec<(RoomId, RoomId)> = (0..4999).map(|i| (i, i + 1)).collect();
        let g = graph(5000, &edges);
        assert_eq!(articulation_points(&g).len(), 4998);
    }
}
