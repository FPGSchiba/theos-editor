//! `merge_hall`: a room grows over free neighbour cells, or absorbs an adjacent connected path room (before space).
use super::Rule;
use crate::graph::{Cell, Graph, Kind, Role, NONE};
use crate::rng::Rng;

pub struct MergeHall { pub sizes: Vec<(i32, i32)> }

impl Rule for MergeHall {
    fn name(&self) -> &'static str { "merge_hall" }
    fn try_apply(&self, g: &mut Graph, rng: &mut Rng) -> bool {
        if g.start == NONE { return false; }
        // matches: (room, rect origin, size) with every other rect cell free; rooms are 1x1 path/leaf rooms
        let mut m: Vec<(usize, Cell, (i32, i32))> = vec![];
        for u in 0..g.nodes.len() {
            let n = &g.nodes[u];
            if n.size != (1, 1) || !matches!(n.role, Role::Path | Role::Leaf) { continue; }
            for &(w, h) in &self.sizes {
                for oy in 0..h { for ox in 0..w {
                    let o = (n.cell.0 - ox, n.cell.1 - oy);
                    let ok = (0..h).all(|dy| (0..w).all(|dx| {
                        let c = (o.0 + dx, o.1 + dy);
                        c == n.cell || g.free(c)
                    }));
                    if ok { m.push((u, o, (w, h))); }
                } }
            }
        }
        // contraction matches: an open edge between two adjacent 1x1 path rooms without a common neighbour
        let mut pairs: Vec<usize> = vec![];
        for (i, e) in g.edges.iter().enumerate() {
            let (a, b) = (&g.nodes[e.a], &g.nodes[e.b]);
            if e.kind != Kind::Open || a.size != (1, 1) || b.size != (1, 1) { continue; }
            if a.role != Role::Path || b.role != Role::Path || (a.key.is_some() && b.key.is_some()) { continue; }
            let common = g.edges.iter().any(|f| { let o = if f.a == e.a { f.b } else if f.b == e.a { f.a } else { return false }; g.connected(o, e.b) && o != e.b });
            if !common { pairs.push(i); }
        }
        let total = m.len() + pairs.len();
        if total == 0 { return false; }
        let k = rng.below(total);
        if k < m.len() {
            let (u, o, (w, h)) = m[k];
            g.nodes[u].cell = o;
            g.nodes[u].size = (w, h);
            for c in g.node_cells(u) { let i = g.idx(c); g.occ[i] = u; }
            true
        } else {
            let e = g.edges.remove(pairs[k - m.len()]);
            contract(g, e.a, e.b);
            true
        }
    }
}

/// Merge room `v` into `u` (adjacent 1x1 rooms): redirect edges, drop `v`, renumber.
fn contract(g: &mut Graph, u: usize, v: usize) {
    let (cu, cv) = (g.nodes[u].cell, g.nodes[v].cell);
    let origin = (cu.0.min(cv.0), cu.1.min(cv.1));
    let size = ((cu.0 - cv.0).abs() + 1, (cu.1 - cv.1).abs() + 1);
    if g.nodes[u].key.is_none() { g.nodes[u].key = g.nodes[v].key; }
    g.nodes[u].cell = origin;
    g.nodes[u].size = size;
    for e in g.edges.iter_mut() {
        if e.a == v { e.a = u; }
        if e.b == v { e.b = u; }
    }
    g.nodes.remove(v);
    let fix = |x: usize| if x > v { x - 1 } else { x };
    for e in g.edges.iter_mut() { e.a = fix(e.a); e.b = fix(e.b); }
    for o in g.occ.iter_mut() { if *o == v { *o = u; } }
    for o in g.occ.iter_mut() { if *o != NONE { *o = fix(*o); } }
    g.start = fix(g.start);
    g.goal = fix(g.goal);
}
