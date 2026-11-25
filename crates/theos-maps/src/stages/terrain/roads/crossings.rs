//! The `bridge` and `ford` sites where roads cross water (spec §5.9, `docs/map-package.md` §5).

use std::collections::BTreeSet;

use super::route::bridge_key;
use super::{Road, StepCost};
use crate::features::{Feature, Prop};
use crate::kernels::num::{round_to, to_u32};
use crate::world::MapFrame;

/// The crossing sites of `roads`, in road order and along each road: a `bridge` at the middle
/// of each bridge with `span_m` and `deck_m`, and a `ford` at the middle stream cell of each
/// stream crossing (the stream cells of a run of steps that each pass one). A bridge or a stream
/// cell that an earlier road crossed gets no second site. Each site's `road` is the index of
/// the road that crossed first.
pub(super) fn sites(cost: &StepCost<'_>, roads: &[Road], frame: &MapFrame) -> Vec<Feature> {
    let mut out = Vec::new();
    let mut bridged = BTreeSet::new();
    let mut forded = BTreeSet::new();
    for (r, road) in roads.iter().enumerate() {
        let id = Prop::Road(to_u32(r));
        let mut run: Vec<usize> = Vec::new();
        for (a, b) in road.steps() {
            let wet: Vec<usize> = if road.is_bridge(a, b) {
                Vec::new()
            } else {
                let entered = cost.entered(a, b);
                entered.into_iter().filter(|&c| cost.is_stream(c)).collect()
            };
            if wet.is_empty() {
                ford(&mut run, &mut forded, &id, frame, &mut out);
            }
            for c in wet {
                if !run.contains(&c) {
                    run.push(c);
                }
            }
            if road.is_bridge(a, b) && bridged.insert(bridge_key(a, b)) {
                out.push(bridge(cost, a, b, &id, frame));
            }
        }
        ford(&mut run, &mut forded, &id, frame, &mut out);
    }
    out
}

/// Ends the stream crossing `run`: a ford at its middle cell unless a cell of it was forded
/// before.
fn ford(
    run: &mut Vec<usize>,
    forded: &mut BTreeSet<usize>,
    id: &Prop,
    frame: &MapFrame,
    out: &mut Vec<Feature>,
) {
    if run.is_empty() {
        return;
    }
    if run.iter().all(|c| !forded.contains(c)) {
        let mid = run[run.len() / 2];
        let at = frame.index_centre_m(mid);
        out.push(Feature::site("ford", at, [("road", id.clone())]));
    }
    forded.extend(run.drain(..));
}

/// The bridge from bank cell `a` to bank cell `b`.
fn bridge(cost: &StepCost<'_>, a: usize, b: usize, id: &Prop, frame: &MapFrame) -> Feature {
    let [xa, ya] = frame.index_centre_m(a);
    let [xb, yb] = frame.index_centre_m(b);
    let tenth = |m: f64| Prop::Num(round_to(m, 10.0));
    let props = [
        ("road", id.clone()),
        ("span_m", tenth(frame.distance_m(a, b))),
        ("deck_m", tenth(cost.deck_m(a, b))),
    ];
    Feature::site(
        "bridge",
        [f64::midpoint(xa, xb), f64::midpoint(ya, yb)],
        props,
    )
}

#[cfg(test)]
mod tests {
    use super::super::cost::tests::{Layers, N, at};
    use super::super::{RoadClass, RoadsParams};
    use super::*;
    use crate::features::{FeatureHandle, Geometry};

    fn road(cells: &[usize], bridges: &[(usize, usize)]) -> Road {
        let cell = |i: usize| u32::try_from(i).unwrap();
        Road {
            class: RoadClass::Track,
            cells: cells.iter().map(|&i| cell(i)).collect(),
            from: FeatureHandle(0),
            to: FeatureHandle(1),
            bridges: bridges.iter().map(|&(a, b)| (cell(a), cell(b))).collect(),
        }
    }

    #[test]
    fn one_ford_per_crossing_and_one_site_per_bridge() {
        let mut layers = Layers::flat(N);
        for r in 0..N {
            layers = layers.wet("river", at(r, 4)).wet("river", at(r, 5));
            layers = layers.wet("big_river", at(r, 10));
        }
        let params = RoadsParams::default();
        let frame = layers.frame();
        let cost = StepCost::new(layers.terrain(), &params, &frame);
        // Across the two-cell stream, then a bridge; a second road crosses both again, back
        // and partly by other cells.
        let first = road(
            &[
                at(2, 3),
                at(2, 4),
                at(2, 5),
                at(2, 6),
                at(2, 7),
                at(2, 8),
                at(2, 9),
                at(2, 11),
            ],
            &[(at(2, 9), at(2, 11))],
        );
        let again = road(
            &[
                at(2, 11),
                at(2, 9),
                at(2, 8),
                at(2, 7),
                at(3, 6),
                at(2, 5),
                at(2, 4),
                at(3, 3),
            ],
            &[(at(2, 11), at(2, 9))],
        );
        let other = road(&[at(8, 3), at(9, 5), at(9, 6)], &[]);
        let found = sites(&cost, &[first, again, other], &frame);
        let summary: Vec<(&str, [f64; 2], Option<&Prop>)> = found
            .iter()
            .map(|f| match f.geometry {
                Geometry::Point(p) => (f.ty.as_str(), p, f.props.get("road")),
                _ => panic!("a site is a point"),
            })
            .collect();
        assert_eq!(
            summary,
            [
                ("ford", [550.0, 250.0], Some(&Prop::Road(0))),
                ("bridge", [1050.0, 250.0], Some(&Prop::Road(0))),
                // The knight move from (8, 3) crosses (8, 4) and (9, 4), then lands on (9, 5).
                ("ford", [450.0, 950.0], Some(&Prop::Road(2))),
            ]
        );
        assert_eq!(found[1].props.get("span_m"), Some(&Prop::Num(200.0)));
        assert_eq!(found[1].props.get("deck_m"), Some(&Prop::Num(103.0)));
    }
}
