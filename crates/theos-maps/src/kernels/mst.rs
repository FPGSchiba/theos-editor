//! Kruskal's minimum spanning tree with a union-find.

use crate::kernels::num::{OrdF64, key};

/// Disjoint sets of `0..n`. The smaller index always becomes the root of a union, so results do
/// not depend on a rank heuristic; path compression keeps `find` fast.
#[derive(Debug, Clone)]
pub struct UnionFind {
    parent: Vec<usize>,
}

impl UnionFind {
    /// `n` singleton sets.
    #[must_use]
    pub fn new(n: usize) -> Self {
        Self {
            parent: (0..n).collect(),
        }
    }

    /// The root of the set holding `x`, which must be below `n`.
    pub fn find(&mut self, x: usize) -> usize {
        let mut root = x;
        while self.parent[root] != root {
            root = self.parent[root];
        }
        let mut at = x;
        while self.parent[at] != root {
            let next = self.parent[at];
            self.parent[at] = root;
            at = next;
        }
        root
    }

    /// Joins the sets of `a` and `b` (both below `n`); `false` when they were already one set.
    pub fn union(&mut self, a: usize, b: usize) -> bool {
        let (ra, rb) = (self.find(a), self.find(b));
        if ra == rb {
            return false;
        }
        self.parent[ra.max(rb)] = ra.min(rb);
        true
    }
}

/// The minimum spanning forest of the graph on `0..n` with `edges` `(a, b, weight)`, as the
/// indices of the chosen edges in the order they were taken. Edges are taken by
/// `(weight, a, b)` and then by their index, so equal weights break ties by cell index. Edges
/// with an end of `n` or more are ignored.
#[must_use]
pub fn kruskal(n: usize, edges: &[(usize, usize, f64)]) -> Vec<usize> {
    let mut order: Vec<usize> = (0..edges.len())
        .filter(|&e| edges[e].0 < n && edges[e].1 < n)
        .collect();
    order.sort_by_key(|&e| (OrdF64(key(edges[e].2)), edges[e].0, edges[e].1));
    let mut sets = UnionFind::new(n);
    order
        .into_iter()
        .filter(|&e| sets.union(edges[e].0, edges[e].1))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mst_of_a_known_graph() {
        let edges = [
            (0, 1, 1.0),
            (1, 2, 2.0),
            (0, 2, 3.0),
            (2, 3, 4.0),
            (1, 3, 5.0),
        ];
        assert_eq!(kruskal(4, &edges), vec![0, 1, 3]);
    }

    #[test]
    fn equal_weights_tie_by_index() {
        let edges = [(1, 2, 1.0), (0, 2, 1.0), (0, 1, 1.0)];
        assert_eq!(kruskal(3, &edges), vec![2, 1]);
        let zeros = [(1, 2, -0.0), (0, 1, 0.0)];
        assert_eq!(kruskal(3, &zeros), vec![1, 0]);
    }

    #[test]
    fn forests_and_bad_edges() {
        let edges = [(0, 9, 1.0), (0, 1, 2.0), (3, 3, 0.5)];
        assert_eq!(kruskal(4, &edges), vec![1]);
        assert_eq!(kruskal(0, &[]), Vec::<usize>::new());
    }

    #[test]
    fn union_find_joins_by_smaller_root() {
        let mut sets = UnionFind::new(5);
        assert!(sets.union(3, 4));
        assert!(!sets.union(4, 3));
        assert_eq!(sets.find(4), 3);
        assert!(sets.union(4, 1));
        assert_eq!(sets.find(3), 1);
    }
}
