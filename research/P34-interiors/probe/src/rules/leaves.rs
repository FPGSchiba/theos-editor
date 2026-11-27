//! `leaf`, `locked_leaf` (key in an earlier room) and `rim_leaf` (extraction room on the envelope rim).
use super::Rule;
use crate::graph::{Cell, Graph, Kind, Role, NONE};
use crate::rng::Rng;

/// `wide`: attach to any non-locked room (round 2) instead of path/start/leaf/goal (round 1).
pub struct LeafRule { pub role: Role, pub rim: bool, pub locked: bool, pub wide: bool }

impl LeafRule {
    pub fn leaf(wide: bool) -> Self { LeafRule { role: Role::Leaf, rim: false, locked: false, wide } }
    pub fn locked_leaf() -> Self { LeafRule { role: Role::LockedLeaf, rim: false, locked: true, wide: false } }
    pub fn rim_leaf(wide: bool) -> Self { LeafRule { role: Role::Extraction, rim: true, locked: false, wide } }
}

fn on_rim(g: &Graph, c: Cell) -> bool { Graph::nb4(c).iter().any(|&n| !g.enabled_at(n)) }

impl Rule for LeafRule {
    fn name(&self) -> &'static str {
        match (self.locked, self.rim) { (true, _) => "locked_leaf", (_, true) => "rim_leaf", _ => "leaf" }
    }
    fn try_apply(&self, g: &mut Graph, rng: &mut Rng) -> bool {
        if g.start == NONE { return false; }
        let mut m: Vec<(usize, Cell, Cell)> = vec![];
        for u in 0..g.nodes.len() {
            let ro = g.nodes[u].role;
            let ok = if self.wide { ro != Role::LockedLeaf } else { matches!(ro, Role::Path | Role::Start | Role::Leaf | Role::Goal) };
            if !ok { continue; }
            for (own, free) in g.slots(u) {
                if self.rim && !on_rim(g, free) { continue; }
                m.push((u, own, free));
            }
        }
        let Some(&(u, own, free)) = rng.pick(&m) else { return false };
        let key_room = if self.locked {
            // earlier rooms: at most as far from the start as the attachment room; any room reachable before the lock
            let d = g.hops(g.start, false);
            let mut ks: Vec<usize> = (0..g.nodes.len()).filter(|&i| g.nodes[i].key.is_none() && g.nodes[i].role != Role::Goal && d[i] <= d[u]).collect();
            if ks.is_empty() { ks = (0..g.nodes.len()).filter(|&i| g.nodes[i].key.is_none() && g.nodes[i].role != Role::Goal).collect(); }
            let Some(&k) = rng.pick(&ks) else { return false };
            Some(k)
        } else { None };
        let n = g.add_node(free, (1, 1), self.role);
        let kind = if let Some(kr) = key_room {
            let k = g.new_key();
            g.nodes[kr].key = Some(k);
            Kind::Locked(k)
        } else { Kind::Open };
        g.add_edge(u, n, own, free, kind, false);
        true
    }
}
