//! Rewriting rules behind one trait; shared helpers for free-cell paths.
use crate::graph::{Cell, Graph, Kind, Role};
use crate::rng::Rng;
use std::cmp::Reverse;
use std::collections::BinaryHeap;

pub mod cycles;
pub mod leaves;
pub mod main_cycle;
pub mod merge;
pub mod shortcut;

pub trait Rule {
    fn name(&self) -> &'static str;
    /// Enumerate matches in a total order, choose one with the seeded stream, rewrite. False = no match.
    fn try_apply(&self, g: &mut Graph, rng: &mut Rng) -> bool;
}

/// Apply a rule; keep the result only if the room graph is still solvable, else roll back.
pub fn apply_guarded(rule: &dyn Rule, g: &mut Graph, rng: &mut Rng) -> bool {
    let snap = g.clone();
    if rule.try_apply(g, rng) {
        if g.check().is_ok() { return true; }
        g.rollbacks += 1;
    }
    let rb = g.rollbacks;
    *g = snap;
    g.rollbacks = rb;
    false
}

/// Randomised-cost Dijkstra over free lattice cells from `src` to any of `goals`.
/// Returns cells `src .. goal`. `blocked` marks extra forbidden cells.
pub fn random_path(g: &Graph, src: Cell, goals: &[Cell], blocked: &[bool], forbid_direct: bool, noise: usize, rng: &mut Rng) -> Option<Vec<Cell>> {
    let n = (g.w * g.h) as usize;
    let cost: Vec<u32> = (0..n).map(|_| 1 + rng.below(noise + 1) as u32).collect();
    let mut dist = vec![u32::MAX; n];
    let mut prev = vec![usize::MAX; n];
    let mut heap = BinaryHeap::new();
    let s = g.idx(src);
    dist[s] = 0;
    heap.push(Reverse((0u32, s)));
    while let Some(Reverse((d, i))) = heap.pop() {
        if d > dist[i] { continue; }
        let c = g.cell_of(i);
        if i != s && goals.contains(&c) {
            let mut p = vec![c];
            let mut j = i;
            while prev[j] != usize::MAX { j = prev[j]; p.push(g.cell_of(j)); }
            p.reverse();
            return Some(p);
        }
        for nb in Graph::nb4(c) {
            if !g.enabled_at(nb) { continue; }
            let ni = g.idx(nb);
            let is_goal = goals.contains(&nb);
            if !is_goal && (!g.free(nb) || blocked[ni]) { continue; }
            if is_goal && forbid_direct && i == s { continue; }
            let nd = d + cost[ni];
            if nd < dist[ni] { dist[ni] = nd; prev[ni] = i; heap.push(Reverse((nd, ni))); }
        }
    }
    None
}

/// Detour of `min..=max` new rooms from `src` cell around to node `dst` (never the direct step).
pub fn find_detour(g: &Graph, src: Cell, dst: usize, min: usize, max: usize, noise: usize, rng: &mut Rng) -> Option<Vec<Cell>> {
    let goals = g.node_cells(dst);
    let blocked = vec![false; (g.w * g.h) as usize];
    for _ in 0..6 {
        let p = random_path(g, src, &goals, &blocked, true, noise, rng)?;
        let interior = p.len() - 2;
        if interior >= min && interior <= max { return Some(p); }
    }
    None
}

/// Create the rooms of a path `p` (p[0] in `u`, last cell in `v`) with open edges. Returns (new nodes, new edges).
pub fn materialise(g: &mut Graph, u: usize, v: usize, p: &[Cell], main: bool) -> (Vec<usize>, Vec<usize>) {
    let mut nodes = vec![];
    let mut edges = vec![];
    let mut prev = u;
    for i in 1..p.len() - 1 {
        let n = g.add_node(p[i], (1, 1), Role::Path);
        edges.push(g.add_edge(prev, n, p[i - 1], p[i], Kind::Open, main));
        nodes.push(n);
        prev = n;
    }
    edges.push(g.add_edge(prev, v, p[p.len() - 2], p[p.len() - 1], Kind::Open, main));
    (nodes, edges)
}
