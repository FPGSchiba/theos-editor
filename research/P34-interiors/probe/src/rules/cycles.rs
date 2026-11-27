//! `insert_cycle` (minor cycle, optional lock+key) and `extend_arc`.
use super::{find_detour, materialise, Rule};
use crate::graph::{Graph, Kind, NONE};
use crate::rng::Rng;

/// Keeps edge u-v and adds a detour. With a lock: u-v gets locked, the key sits on the detour
/// (entrance side u), and the detour's last edge is a one-way drop from v back into the detour.
/// `close`: if no free-cell detour fits, close a loop around occupied nodes with an open edge between two
/// adjacent unconnected rooms whose graph distance is 3..=`max_rooms`+1 (2x2, 2x3, 3x3 rings).
pub struct InsertCycle { pub lock_pct: u32, pub max_rooms: usize, pub noise: usize, pub close: bool }

impl Rule for InsertCycle {
    fn name(&self) -> &'static str { "insert_cycle" }
    fn try_apply(&self, g: &mut Graph, rng: &mut Rng) -> bool {
        if g.start == NONE { return false; }
        let dist = g.hops(g.start, false);
        let mut cands: Vec<usize> = (0..g.edges.len()).filter(|&i| g.edges[i].kind == Kind::Open).collect();
        rng.shuffle(&mut cands);
        let lock = rng.pct(self.lock_pct);
        for &ei in cands.iter().take(10) {
            let e = g.edges[ei].clone();
            let (u, v, cu) = if dist[e.a] <= dist[e.b] { (e.a, e.b, e.ca) } else { (e.b, e.a, e.cb) };
            let lock = lock && dist[e.a] != dist[e.b];
            let Some(p) = find_detour(g, cu, v, 2, self.max_rooms, self.noise, rng) else { continue };
            let (nodes, edges) = materialise(g, u, v, &p, false);
            if lock {
                let k = g.new_key();
                g.edges[ei].kind = Kind::Locked(k);
                let last = *edges.last().unwrap();
                let l = &mut g.edges[last];
                std::mem::swap(&mut l.a, &mut l.b);
                std::mem::swap(&mut l.ca, &mut l.cb);
                l.kind = Kind::Drop;
                let kn = nodes[rng.below(nodes.len())];
                g.nodes[kn].key = Some(k);
            }
            return true;
        }
        if self.close { return self.close_loop(g, rng); }
        false
    }
}

impl InsertCycle {
    fn close_loop(&self, g: &mut Graph, rng: &mut Rng) -> bool {
        let mut m = vec![];
        for a in 0..g.nodes.len() {
            for c in g.node_cells(a) { for nb in Graph::nb4(c) {
                if !g.inb(nb) { continue; }
                let b = g.occ[g.idx(nb)];
                if b == NONE || b <= a || g.connected(a, b) { continue; }
                m.push((a, b, c, nb));
            } }
        }
        rng.shuffle(&mut m);
        for (a, b, ca, cb) in m.into_iter().take(30) {
            let d = g.hops(a, false)[b];
            if d >= 3 && d as usize <= self.max_rooms + 1 {
                g.add_edge(a, b, ca, cb, Kind::Open, false);
                return true;
            }
        }
        false
    }
}

/// Replaces an open edge u-v by a longer path through free cells.
pub struct ExtendArc { pub max_rooms: usize, pub noise: usize }

impl Rule for ExtendArc {
    fn name(&self) -> &'static str { "extend_arc" }
    fn try_apply(&self, g: &mut Graph, rng: &mut Rng) -> bool {
        let mut cands: Vec<usize> = (0..g.edges.len()).filter(|&i| g.edges[i].kind == Kind::Open).collect();
        rng.shuffle(&mut cands);
        for &ei in cands.iter().take(10) {
            let e = g.edges[ei].clone();
            let Some(p) = find_detour(g, e.ca, e.b, 2, self.max_rooms, self.noise, rng) else { continue };
            g.edges.remove(ei);
            materialise(g, e.a, e.b, &p, e.main);
            return true;
        }
        false
    }
}
