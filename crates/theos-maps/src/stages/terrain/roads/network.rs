//! Choosing the links of the network: the Delaunay candidates, their routed costs, Kruskal's
//! tree and the loops, and the order in which they are laid.

use std::cmp::Reverse;

use super::places::{Place, class_of, may_loop};
use super::{RoadClass, Router};
use crate::kernels::KernelError;
use crate::kernels::mst::kruskal;
use crate::kernels::num::{OrdF64, key};
use crate::kernels::voronoi::delaunay_edges;

/// What the network's construction reports to its caller and asks of it.
pub(super) trait Watch {
    /// Whether the run was cancelled.
    fn cancelled(&self) -> bool;
    /// `links` more links are routed (or settled without routing).
    fn advance(&mut self, links: usize);
}

/// A candidate link between the places `a < b` (indices into the place list), with the class
/// its ends give and its routed cost.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct Link {
    /// The first place.
    pub a: usize,
    /// The second place.
    pub b: usize,
    /// The class.
    pub class: RoadClass,
    /// The routed cost from `a` to `b`, without the merge discount.
    pub cost: f64,
}

/// The candidate pairs `(a, b)`, `a < b`: the Delaunay edges over the places' points.
///
/// # Errors
/// [`KernelError::InvalidPoint`] for a point the triangulation refuses.
pub(super) fn candidates(places: &[Place]) -> Result<Vec<(usize, usize)>, KernelError> {
    let points: Vec<[f64; 2]> = places.iter().map(|p| p.at).collect();
    delaunay_edges(&points)
}

/// Routes every candidate pair from its first place to its second with the class its ends give,
/// one search per first place and class, and returns those a road can join, in pair order.
/// `watch` advances by the pairs each search costs.
///
/// # Errors
/// [`KernelError::Cancelled`] when `watch` says so during a search.
pub(super) fn cost(
    router: &Router<'_>,
    places: &[Place],
    pairs: &[(usize, usize)],
    watch: &mut dyn Watch,
) -> Result<Vec<Link>, KernelError> {
    let mut wanted: Vec<(usize, RoadClass, usize)> = pairs
        .iter()
        .map(|&(a, b)| (a, class_of(&places[a], &places[b]), b))
        .collect();
    wanted.sort_unstable();
    let mut links = Vec::new();
    for group in wanted.chunk_by(|x, y| (x.0, x.1) == (y.0, y.1)) {
        let (a, class, _) = group[0];
        let tree = router.tree(&[places[a].cell], class, &|| watch.cancelled())?;
        for &(_, _, b) in group {
            let cost = tree.dist[places[b].cell];
            if cost.is_finite() {
                links.push(Link { a, b, class, cost });
            }
        }
        watch.advance(group.len());
    }
    links.sort_unstable_by_key(|l| (l.a, l.b));
    Ok(links)
}

/// The links to lay: Kruskal's tree of `links` over `n` places, then, in order of cost, each
/// other link between two places that may loop whose cost through the links taken so far
/// exceeds `loop_detour` times its own.
pub(super) fn choose(places: &[Place], links: &[Link], loop_detour: f64) -> Vec<Link> {
    let edges: Vec<(usize, usize, f64)> = links.iter().map(|l| (l.a, l.b, l.cost)).collect();
    let mut taken = vec![false; links.len()];
    for i in kruskal(places.len(), &edges) {
        taken[i] = true;
    }
    let mut others: Vec<usize> = (0..links.len())
        .filter(|&i| !taken[i] && may_loop(&places[links[i].a], &places[links[i].b]))
        .collect();
    others.sort_unstable_by_key(|&i| (OrdF64(key(links[i].cost)), links[i].a, links[i].b));
    for i in others {
        let so_far: Vec<&Link> = links
            .iter()
            .zip(&taken)
            .filter_map(|(l, &t)| t.then_some(l))
            .collect();
        let through = network_cost(places.len(), &so_far, links[i].a, links[i].b);
        if through > loop_detour * links[i].cost {
            taken[i] = true;
        }
    }
    links
        .iter()
        .zip(&taken)
        .filter_map(|(l, &t)| t.then_some(*l))
        .collect()
}

/// `links` in the order they are laid: by class, main first, then by cost, then by their
/// places (whose order is their handles').
pub(super) fn laying_order(mut links: Vec<Link>) -> Vec<Link> {
    links.sort_unstable_by_key(|l| (Reverse(l.class), OrdF64(key(l.cost)), l.a, l.b));
    links
}

/// The least cost from place `a` to place `b` over `links` among `n` places; infinite when no
/// links join them.
fn network_cost(n: usize, links: &[&Link], a: usize, b: usize) -> f64 {
    let mut dist = vec![f64::INFINITY; n];
    let mut done = vec![false; n];
    dist[a] = 0.0;
    // Dijkstra on a handful of places: the nearest open place, ties by index.
    while let Some(at) = (0..n)
        .filter(|&i| !done[i] && dist[i].is_finite())
        .min_by_key(|&i| (OrdF64(key(dist[i])), i))
    {
        if at == b {
            break;
        }
        done[at] = true;
        for l in links {
            let other = if l.a == at {
                l.b
            } else if l.b == at {
                l.a
            } else {
                continue;
            };
            let d = key(dist[at] + l.cost);
            if d < dist[other] {
                dist[other] = d;
            }
        }
    }
    dist[b]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::features::FeatureHandle;

    fn place(i: u32, rank: u8, settlement: bool) -> Place {
        Place {
            handle: FeatureHandle(i),
            at: [f64::from(i) * 100.0, f64::from(i * i) * 10.0],
            cell: 0,
            rank,
            settlement,
            label: String::new(),
        }
    }

    fn link(a: usize, b: usize, cost: f64) -> Link {
        Link {
            a,
            b,
            class: RoadClass::Main,
            cost,
        }
    }

    #[test]
    fn the_tree_takes_the_cheapest_links_and_loops_join_on_long_detours() {
        let towns: Vec<Place> = (0..4).map(|i| place(i, 3, true)).collect();
        // A square 0-1-2-3 with sides of 10 and a diagonal 0-2 of 12, in pair order.
        let links = [
            link(0, 1, 10.0),
            link(0, 2, 12.0),
            link(0, 3, 10.0),
            link(1, 2, 10.0),
            link(2, 3, 10.0),
        ];
        let pairs = |chosen: &[Link]| chosen.iter().map(|l| (l.a, l.b)).collect::<Vec<_>>();
        // The tree is 0-1, 0-3, 1-2 (equal costs tie by ends). Through it 2-3 costs 30, more
        // than 1.7 · 10, so it joins; then 0-2 costs 20 through 0-1-2, less than 1.7 · 12.
        let chosen = choose(&towns, &links, 1.7);
        assert_eq!(pairs(&chosen), [(0, 1), (0, 3), (1, 2), (2, 3)]);
        let tree = [(0, 1), (0, 3), (1, 2)];
        assert_eq!(pairs(&choose(&towns, &links, 3.5)), tree);
        let hamlets: Vec<Place> = (0..4).map(|i| place(i, 1, true)).collect();
        assert_eq!(pairs(&choose(&hamlets, &links, 1.0)), tree);
        let gates: Vec<Place> = (0..4).map(|i| place(i, 3, false)).collect();
        assert_eq!(pairs(&choose(&gates, &links, 1.0)), tree);
    }

    #[test]
    fn links_are_laid_main_first_then_by_cost_then_by_ends() {
        let mut road = link(0, 1, 5.0);
        road.class = RoadClass::Road;
        let order = laying_order(vec![
            road,
            link(2, 3, 9.0),
            link(1, 2, 9.0),
            link(0, 2, 7.0),
        ]);
        let pairs: Vec<(usize, usize)> = order.iter().map(|l| (l.a, l.b)).collect();
        assert_eq!(pairs, [(0, 2), (1, 2), (2, 3), (0, 1)]);
    }

    #[test]
    fn network_costs_follow_the_cheapest_way() {
        let links = [link(0, 1, 2.0), link(1, 2, 2.0), link(0, 2, 5.0)];
        let refs: Vec<&Link> = links.iter().collect();
        assert!((network_cost(4, &refs, 0, 2) - 4.0).abs() < 1e-12);
        assert!((network_cost(4, &refs, 2, 0) - 4.0).abs() < 1e-12);
        assert!(network_cost(4, &refs, 0, 3).is_infinite());
    }

    #[test]
    fn two_places_are_one_candidate() -> Result<(), KernelError> {
        let two = [place(1, 3, true), place(2, 3, true)];
        assert_eq!(candidates(&two)?, [(0, 1)]);
        let three = [place(1, 3, true), place(2, 3, true), place(3, 1, true)];
        assert_eq!(candidates(&three)?, [(0, 1), (0, 2), (1, 2)]);
        Ok(())
    }
}
