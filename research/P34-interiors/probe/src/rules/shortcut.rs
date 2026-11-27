//! `shortcut`: a one-way edge from a deeper room to an adjacent shallower room where the forward route is long.
use super::Rule;
use crate::graph::{Graph, Kind, NONE};
use crate::rng::Rng;

pub struct Shortcut { pub min_hops: u32 }

impl Rule for Shortcut {
    fn name(&self) -> &'static str { "shortcut" }
    fn try_apply(&self, g: &mut Graph, rng: &mut Rng) -> bool {
        if g.start == NONE { return false; }
        let depth = g.hops(g.start, false);
        // matches: adjacent, unconnected pairs (a < b) in coordinate order; direction deep -> shallow
        let mut m = vec![];
        for a in 0..g.nodes.len() {
            for c in g.node_cells(a) { for nb in Graph::nb4(c) {
                if !g.inb(nb) { continue; }
                let b = g.occ[g.idx(nb)];
                if b == NONE || b <= a || g.connected(a, b) || depth[a] == depth[b] { continue; }
                let (d, s, cd, cs) = if depth[a] > depth[b] { (a, b, c, nb) } else { (b, a, nb, c) };
                m.push((d, s, cd, cs));
            } }
        }
        m.sort_by_key(|x| (x.0, x.1));
        m.dedup_by_key(|x| (x.0, x.1));
        rng.shuffle(&mut m);
        for (d, s, cd, cs) in m.into_iter().take(25) {
            let fwd = g.hops(d, true);
            if fwd[s] != u32::MAX && fwd[s] >= self.min_hops {
                g.add_edge(d, s, cd, cs, Kind::Drop, false);
                return true;
            }
        }
        false
    }
}
