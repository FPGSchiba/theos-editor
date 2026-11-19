//! The sites of work: mills on rivers near towns, and mines, quarries, lumber camps and shrines
//! on deposits.

use std::collections::BTreeSet;

use crate::features::{Feature, FeatureSet, Geometry, Prop};
use crate::grid::Grid;
use crate::stages::terrain::roads::RoadClass;
use crate::world::MapFrame;

/// How far from a settlement its mill may stand, m (0.3.0's 1.5 km).
const MILL_REACH_M: f64 = 1500.0;

/// What a mill needs of the map.
pub(super) struct River<'a> {
    /// River cells.
    pub cells: &'a Grid<bool>,
    /// Discharge, m³/s.
    pub discharge: &'a Grid<f64>,
    /// The least discharge of a mill, m³/s.
    pub min_discharge: f64,
    /// The cells away from the edge.
    pub inside: &'a Grid<bool>,
}

/// The mills: for each town, village or city in handle order, the river cell with enough
/// discharge nearest to it within [`MILL_REACH_M`], ties to the lowest index; no two mills on a
/// cell.
pub(super) fn mills(set: &FeatureSet, water: &River<'_>, frame: &MapFrame) -> Vec<Feature> {
    let mut taken = BTreeSet::new();
    let mut out = Vec::new();
    for (handle, s) in set.of_kind("settlement") {
        let Geometry::Point([x, y]) = s.geometry else {
            continue;
        };
        if !matches!(s.ty.as_str(), "town" | "village" | "city") {
            continue;
        }
        let mut best: Option<(usize, f64)> = None;
        for i in 0..water.cells.len() {
            let ok = water.cells.as_slice()[i]
                && water.discharge.as_slice()[i] >= water.min_discharge
                && water.inside.as_slice()[i]
                && !taken.contains(&i);
            if !ok {
                continue;
            }
            let (row, col) = water.cells.cell(i);
            let [cx, cy] = frame.centre_m(row, col);
            let d2 = (cx - x) * (cx - x) + (cy - y) * (cy - y);
            if d2 < MILL_REACH_M * MILL_REACH_M && best.is_none_or(|(_, b)| d2 < b) {
                best = Some((i, d2));
            }
        }
        if let Some((i, _)) = best {
            taken.insert(i);
            let (row, col) = water.cells.cell(i);
            out.push(Feature::site(
                "mill",
                frame.centre_m(row, col),
                [("serves", Prop::Ref(handle))],
            ));
        }
    }
    out
}

/// The site type of a deposit type and the class of the spur to it (0.3.0: a track, a trail to a
/// shrine), `None` for deposits that are not worked.
fn worked(ty: &str) -> Option<(&'static str, RoadClass)> {
    match ty {
        "iron-ore" | "copper-ore" | "silver-ore" => Some(("mine", RoadClass::Track)),
        "stone" => Some(("quarry", RoadClass::Track)),
        "timber" => Some(("lumber-camp", RoadClass::Track)),
        "resonance-crystal" => Some(("shrine", RoadClass::Trail)),
        _ => None,
    }
}

/// The sites of the deposits in handle order, each with the class of its spur: those of a worked
/// type with a richness of at least `min_richness`, inside the margin, at the deposit's point
/// and linked to it by `deposit`.
pub(super) fn deposit_sites(
    set: &FeatureSet,
    min_richness: f64,
    inside: &Grid<bool>,
    frame: &MapFrame,
) -> Vec<(Feature, RoadClass)> {
    set.of_kind("resource")
        .filter_map(|(handle, f)| {
            let (ty, class) = worked(&f.ty)?;
            let Some(Prop::Num(richness)) = f.props.get("richness") else {
                return None;
            };
            let Geometry::Point(at) = f.geometry else {
                return None;
            };
            let (row, col) = frame.cell_at_m(at[0], at[1]);
            let near = *richness >= min_richness && inside.get(row, col);
            near.then(|| {
                (
                    Feature::site(ty, at, [("deposit", Prop::Ref(handle))]),
                    class,
                )
            })
        })
        .collect()
}
