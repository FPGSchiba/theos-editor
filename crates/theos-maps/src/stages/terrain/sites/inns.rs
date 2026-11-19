//! Inns at the midpoint of long main and road links, away from settlements.

use crate::features::{Feature, FeatureSet, Prop};
use crate::grid::Grid;
use crate::stages::terrain::roads::{Road, RoadClass, distance_m};
use crate::world::MapFrame;

/// A link is long enough for an inn above this length, m (0.3.0's 6 km).
const MIN_LINK_M: f64 = 6000.0;
/// An inn stands farther than this from every settlement, m (0.3.0's 2 km).
const FROM_SETTLEMENT_M: f64 = 2000.0;

/// The inns of the roads of class main and road longer than 6 km, in road order: one at the
/// path cell nearest to half the road's length (ties to the earlier cell), when it is inside the
/// margin and more than 2 km from every settlement. Its `road` is the road's index.
pub(super) fn inns(
    roads: &[Road],
    set: &FeatureSet,
    inside: &Grid<bool>,
    frame: &MapFrame,
) -> Vec<Feature> {
    let settlements = set.points("settlement", None);
    let mut out = Vec::new();
    for (i, road) in roads.iter().enumerate() {
        let link = matches!(road.class, RoadClass::Main | RoadClass::Road);
        if !link || road.length_m(frame) <= MIN_LINK_M {
            continue;
        }
        let cell = midpoint(road, frame);
        let (row, col) = (cell / frame.cols, cell % frame.cols);
        let at = frame.centre_m(row, col);
        let alone = settlements.iter().all(|s| {
            let (dx, dy) = (s[0] - at[0], s[1] - at[1]);
            dx * dx + dy * dy > FROM_SETTLEMENT_M * FROM_SETTLEMENT_M
        });
        if alone && inside.get(row, col) {
            let id = u32::try_from(i).unwrap_or(u32::MAX);
            out.push(Feature::site("inn", at, [("road", Prop::Road(id))]));
        }
    }
    out
}

/// The path cell of `road` whose distance along the road is nearest to half its length.
fn midpoint(road: &Road, frame: &MapFrame) -> usize {
    let half = road.length_m(frame) / 2.0;
    let mut along = 0.0;
    let first = road
        .cells
        .first()
        .map_or(0, |&c| usize::try_from(c).unwrap_or(0));
    let mut best = (first, half);
    for (a, b) in road.steps() {
        along += distance_m(frame, a, b);
        let off = (along - half).abs();
        if off < best.1 {
            best = (b, off);
        }
    }
    best.0
}
