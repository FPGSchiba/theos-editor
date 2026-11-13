//! The placing of deposits on the suitability fields, 0.3.0's second half of
//! `features.py::resources`.
//!
//! For each type with a count, in the order of [`NAMES`](super::fields::NAMES): the field is
//! smoothed over 93.75 m (0.3.0's 1.5 cells), zeroed outside the frame's margin and multiplied by
//! `1 + 0.25 u`; the best cells at least the spacing apart are picked. A deposit's richness is
//! `clip(0.2 + 0.75 (f / (top + 1e-9))^2 (0.45 + 0.55 u), 0, 1)` with `f` the cell's value and
//! `top` the field's largest; its tier is `exceptional` above 0.8, `rich` above 0.5 and `common`
//! otherwise; its radius is `100 + 300 richness` m, truncated; its element is the strongest
//! element at the cell (ties go to the first of water, wood, earth, metal, fire); its harvest
//! verb depends on the type.
//!
//! Draws, per type with a count in field order: one noise value per cell in index order, then
//! one value per pick in pick order (the richness).

use std::collections::BTreeMap;

use crate::features::{Feature, Geometry, Prop, interior, pick};
use crate::grid::Grid;
use crate::kernels::filters::gaussian;
use crate::kernels::num::{canonical, max, min, round_half_even, to_index};
use crate::rng::StageRng;
use crate::world::MapFrame;

use super::ResourcesParams;

/// Sigma of the smoothing of a field before the picking, 0.3.0's 1.5 cells of 62.5 m.
const FIELD_SIGMA_M: f64 = 93.75;

/// The element names, in the order of the element rasters.
const ELEMENTS: [&str; 5] = ["water", "wood", "earth", "metal", "fire"];

/// The harvest verb of a resource type.
fn harvest(ty: &str) -> &'static str {
    match ty {
        "iron-ore" | "copper-ore" | "silver-ore" => "mine",
        "stone" => "quarry",
        "clay" => "dig",
        "timber" => "fell",
        "peat-reeds" => "cut",
        "salt" => "pan",
        "resonance-crystal" => "attune",
        _ => "gather",
    }
}

/// The richness of a deposit from its field value `f`, the field's largest `top` and a draw `u`.
fn richness(f: f64, top: f64, u: f64) -> f64 {
    let share = f / (top + 1e-9);
    min(
        max(0.2 + 0.75 * (share * share) * (0.45 + 0.55 * u), 0.0),
        1.0,
    )
}

/// The tier of a richness.
fn tier(richness: f64) -> &'static str {
    if richness > 0.8 {
        "exceptional"
    } else if richness > 0.5 {
        "rich"
    } else {
        "common"
    }
}

/// The strongest element at cell `i`; ties go to the first.
fn strongest(elements: &[&Grid<f64>; 5], i: usize) -> &'static str {
    let mut best = (0, elements[0].as_slice()[i]);
    for (k, grid) in elements.iter().enumerate().skip(1) {
        if grid.as_slice()[i] > best.1 {
            best = (k, grid.as_slice()[i]);
        }
    }
    ELEMENTS[best.0]
}

/// One deposit of type `ty` at the centre of cell `(row, col)`.
fn deposit(
    ty: &str,
    cell: (usize, usize),
    frame: &MapFrame,
    rich: f64,
    element: &'static str,
) -> Feature {
    let mut props = BTreeMap::new();
    let rounded = canonical(round_half_even(rich * 100.0) / 100.0);
    props.insert("richness".to_owned(), Prop::Num(rounded));
    props.insert("tier".to_owned(), Prop::Text(tier(rich).to_owned()));
    props.insert("element".to_owned(), Prop::Text(element.to_owned()));
    props.insert("harvest".to_owned(), Prop::Text(harvest(ty).to_owned()));
    let radius = to_index(100.0 + 300.0 * rich, usize::MAX);
    props.insert(
        "radius_m".to_owned(),
        Prop::Int(i64::try_from(radius).unwrap_or(i64::MAX)),
    );
    Feature {
        kind: "resource",
        ty: ty.to_owned(),
        geometry: Geometry::Point(frame.centre_m(cell.0, cell.1)),
        props,
    }
}

/// The deposits to add: for each field with a count, its picks.
pub fn place(
    fields: &[(&'static str, Grid<f64>)],
    elements: &[&Grid<f64>; 5],
    frame: &MapFrame,
    params: &ResourcesParams,
    rng: &mut StageRng,
) -> Vec<Feature> {
    let inside = interior(frame, params.edge_margin);
    let spacing_m = params.spacing_km * 1000.0;
    let mut out = Vec::new();
    for (ty, field) in fields {
        let n = params.counts.get(*ty).copied().unwrap_or(0);
        if n == 0 {
            continue;
        }
        let mut f = gaussian(field, FIELD_SIGMA_M, frame.cell_m);
        for (v, &keep) in f.as_mut_slice().iter_mut().zip(inside.as_slice()) {
            let noise = 1.0 + 0.25 * rng.uniform();
            *v = if keep { *v * noise } else { 0.0 };
        }
        let top = f
            .as_slice()
            .iter()
            .fold(f64::NEG_INFINITY, |m, &v| max(m, v));
        for (row, col) in pick(&f, n, spacing_m, &[], 0.0, frame) {
            let i = f.index(row, col);
            let rich = richness(f.as_slice()[i], top, rng.uniform());
            out.push(deposit(ty, (row, col), frame, rich, strongest(elements, i)));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn near(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-12
    }

    /// A narrow tall block against a broad lower one: the smoothing over 93.75 m (about one cell
    /// here) lets the first win, a wider smoothing the second.
    #[test]
    fn the_field_is_smoothed_before_the_pick() {
        let frame = MapFrame {
            rows: 64,
            cols: 64,
            cell_m: 100.0,
        };
        let mut field = Grid::filled(64, 64, 0.0);
        for r in 10..13 {
            for c in 10..13 {
                field.set(r, c, 1.0);
            }
        }
        for r in 30..39 {
            for c in 30..39 {
                field.set(r, c, 0.6);
            }
        }
        let zero = Grid::filled(64, 64, 0.0);
        let elements = [&zero, &zero, &zero, &zero, &zero];
        let mut params = ResourcesParams::default();
        params.counts.insert("stone".to_owned(), 1);
        let mut rng = StageRng::for_key(1, "test");
        let found = place(&[("stone", field)], &elements, &frame, &params, &mut rng);
        let Some(Geometry::Point([x, y])) = found.first().map(|f| f.geometry.clone()) else {
            panic!("one deposit")
        };
        let (r, c) = frame.cell_at_m(x, y);
        assert!((10..13).contains(&r) && (10..13).contains(&c), "{r},{c}");
    }

    #[test]
    fn richness_follows_the_formula() {
        // 0.2 + 0.75 * 0.5^2 * (0.45 + 0.55 * 0.2) = 0.2 + 0.1875 * 0.56 = 0.305 (top = 1).
        assert!((richness(0.5, 1.0, 0.2) - 0.305).abs() < 1e-6);
        assert!(near(richness(0.0, 1.0, 0.7), 0.2));
        assert!(richness(1.0, 0.0, 1.0) <= 1.0);
        // 0.2 + 0.75 * 1 * 1 = 0.95 at the top with the highest draw.
        assert!((richness(1.0, 1.0, 1.0) - 0.95).abs() < 1e-8);
    }

    #[test]
    fn tiers_split_at_half_and_four_fifths() {
        assert_eq!(tier(0.81), "exceptional");
        assert_eq!(tier(0.8), "rich");
        assert_eq!(tier(0.51), "rich");
        assert_eq!(tier(0.5), "common");
        assert_eq!(tier(0.2), "common");
    }

    #[test]
    fn every_type_has_a_harvest_verb() {
        for (ty, verb) in [
            ("iron-ore", "mine"),
            ("copper-ore", "mine"),
            ("silver-ore", "mine"),
            ("stone", "quarry"),
            ("clay", "dig"),
            ("timber", "fell"),
            ("peat-reeds", "cut"),
            ("salt", "pan"),
            ("resonance-crystal", "attune"),
            ("herbs", "gather"),
        ] {
            assert_eq!(harvest(ty), verb);
        }
    }
}
