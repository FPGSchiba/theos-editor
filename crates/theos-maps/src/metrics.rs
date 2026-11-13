//! The summary paths that exist whatever stages a recipe holds (spec §5.8).

/// The summary paths a constraint may name besides `features.<type>` and `stages.<id>.<metric>`.
/// `travel.chime_to_chime_min` counts only when a `travel` stage is planned.
pub const PATHS: &[&str] = &[
    "elevation_m.relief",
    "lake_pct",
    "rivers_pct",
    "vegetation_cover_pct",
    "precip_mm_yr.median",
    "travel.chime_to_chime_min",
];

use serde_json::{Map, Value};

use crate::grid::Grid;
use crate::kernels::num::{canonical, round_half_even, to_f64};
use crate::kernels::stats::median;
use crate::stage::Metrics;
use crate::stages::terrain::climate::{self, Climate};
use crate::world::World;

/// `x` rounded to one decimal place (`scale` 10) or two (`scale` 100) the way Python's
/// `round(x, n)` does: halves to even on the scaled value, both steps exact IEEE operations
/// (ruling: the plan said `f64::round`, which rounds halves away from zero and would differ from
/// 0.3.0).
///
/// Python's `round` is correctly rounded on the decimal value of `x`; this differs only where
/// `x * scale` itself rounds onto an exact .5, which share-of-cells inputs do not hit.
fn round_to(x: f64, scale: f64) -> f64 {
    canonical(round_half_even(x * scale) / scale)
}

/// The share of `true` cells in percent: `count / cells * 100`, as numpy's `mean() * 100`.
fn share_pct(cells: impl Iterator<Item = bool>, total: usize) -> f64 {
    to_f64(cells.filter(|&c| c).count()) / to_f64(total) * 100.0
}

/// The lowest and highest value of a grid; `None` when it is empty.
fn extent(grid: &Grid<f64>) -> Option<(f64, f64)> {
    let mut it = grid.as_slice().iter().copied();
    let first = it.next()?;
    Some(it.fold((first, first), |(lo, hi), v| {
        (
            crate::kernels::num::min(lo, v),
            crate::kernels::num::max(hi, v),
        )
    }))
}

/// The precipitation field in mm/yr: `climate.fields.precipitation` when the climate holds it,
/// else the fields computed from the height and not written (ruling P2, as `elements` does).
fn precipitation(world: &World) -> Option<Grid<f64>> {
    let climate = world.value::<Climate>("climate")?;
    if let Some(fields) = &climate.fields {
        return Some(fields.precipitation.clone());
    }
    let height = world.f64("height")?;
    Some(climate::fields(height, world.frame(), &climate.params).precipitation)
}

/// The parity metrics of a finished world, as an object keyed by dotted summary path (0.3.0's
/// definitions, `terrain.py`): `elevation_m.relief` is `int(max - min)` of `height` (truncated,
/// line 234); `lake_pct` and `rivers_pct` are the shares of `lake` cells and of `river` cells
/// outside lakes over all cells, two decimals (lines 282 and 284); `vegetation_cover_pct` is the
/// share of `vegetation > 0.5`, one decimal (line 299); `precip_mm_yr.median` is
/// `int(median(precipitation))` (line 280); `travel.chime_to_chime_min` is the `travel` stage's
/// metric, one decimal. A metric whose input is absent is left out, never 0.
#[cfg_attr(
    not(test),
    expect(dead_code, reason = "wired into the runner by Task 41")
)]
pub(crate) fn compute(world: &World, travel: Option<&Metrics>) -> Value {
    let mut out = Map::new();
    let mut put = |path: &str, value: f64| {
        out.insert(path.to_owned(), Value::from(value));
    };
    if let Some((lo, hi)) = world.f64("height").and_then(extent) {
        put("elevation_m.relief", canonical((hi - lo).trunc()));
    }
    let lake = world.mask("lake");
    if let Some(lake) = lake {
        let pct = share_pct(lake.as_slice().iter().copied(), lake.len());
        put("lake_pct", round_to(pct, 100.0));
    }
    if let Some(river) = world.mask("river") {
        let lakes = lake.map(Grid::as_slice);
        let wet = river
            .as_slice()
            .iter()
            .enumerate()
            .map(|(i, &r)| r && !lakes.is_some_and(|l| l[i]));
        put("rivers_pct", round_to(share_pct(wet, river.len()), 100.0));
    }
    if let Some(vegetation) = world.f64("vegetation") {
        let covered = vegetation.as_slice().iter().map(|&v| v > 0.5);
        put(
            "vegetation_cover_pct",
            round_to(share_pct(covered, vegetation.len()), 10.0),
        );
    }
    if let Some(m) = precipitation(world).and_then(|p| median(p.as_slice()).ok()) {
        put("precip_mm_yr.median", canonical(m.trunc()));
    }
    if let Some(&minutes) = travel.and_then(|t| t.0.get("chime_to_chime_min")) {
        put("travel.chime_to_chime_min", round_to(minutes, 10.0));
    }
    Value::Object(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::grid::Grid;
    use crate::stages::terrain::climate::{Climate, ClimateFields, ClimateParams};
    use crate::world::{Layer, MapFrame};

    const N: usize = 4;

    fn grid(f: impl Fn(usize) -> f64) -> Grid<f64> {
        Grid::from_vec(N, N, (0..N * N).map(f).collect()).unwrap()
    }

    fn mask(cells: &[usize]) -> Layer {
        Layer::Mask(Grid::from_vec(N, N, (0..N * N).map(|i| cells.contains(&i)).collect()).unwrap())
    }

    /// A 4 x 4 world: relief 1134.9 m, 1 lake cell, 5 river cells (one of them in the lake),
    /// 1 vegetated cell and a precipitation whose middle values are 700 and 803.
    fn world(with_climate: bool, with_vegetation: bool) -> World {
        let mut w = World::new(MapFrame {
            rows: N,
            cols: N,
            cell_m: 100.0,
        });
        let height = grid(|i| {
            if i == 5 {
                1234.9
            } else {
                100.0 + crate::kernels::num::to_f64(i)
            }
        });
        w.insert("height", Layer::F64(height)).unwrap();
        w.insert("lake", mask(&[0])).unwrap();
        w.insert("river", mask(&[0, 1, 2, 3, 4])).unwrap();
        if with_vegetation {
            w.insert(
                "vegetation",
                Layer::F64(grid(|i| if i == 7 { 0.9 } else { 0.5 })),
            )
            .unwrap();
        }
        if with_climate {
            let precipitation =
                grid(|i| 100.0 * crate::kernels::num::to_f64(i) + if i >= 8 { 3.0 } else { 0.0 });
            let zero = grid(|_| 0.0);
            let climate = Climate {
                params: ClimateParams::default(),
                fields: Some(ClimateFields {
                    precipitation,
                    pet: zero.clone(),
                    aet: zero.clone(),
                    runoff: zero,
                }),
            };
            w.insert("climate", Layer::Value(Box::new(climate)))
                .unwrap();
        }
        w
    }

    fn num(v: &Value, key: &str) -> f64 {
        v.get(key).and_then(Value::as_f64).unwrap_or(f64::NAN)
    }

    #[test]
    fn metrics_follow_0_3_0s_definitions() {
        let m = compute(&world(true, true), None);
        // int(max - min): truncated, not rounded (terrain.py:234).
        assert!((num(&m, "elevation_m.relief") - 1134.0).abs() < 1e-12);
        assert!((num(&m, "lake_pct") - 6.25).abs() < 1e-12);
        // 1 vegetated cell of 16 is 6.25, rounded half to even at one decimal (terrain.py:299).
        assert!((num(&m, "vegetation_cover_pct") - 6.2).abs() < 1e-12);
        // int(median): middle values 700 and 803, 751.5 -> truncated.
        assert!((num(&m, "precip_mm_yr.median") - 751.0).abs() < 1e-12);
    }

    #[test]
    fn a_share_with_a_third_decimal_rounds_to_two() {
        let mut w = World::new(MapFrame {
            rows: 3,
            cols: 3,
            cell_m: 100.0,
        });
        let cells = (0..9).map(|i| i < 3).collect();
        w.insert("lake", Layer::Mask(Grid::from_vec(3, 3, cells).unwrap()))
            .unwrap();
        // 3 / 9 * 100 = 33.333...
        assert!((num(&compute(&w, None), "lake_pct") - 33.33).abs() < 1e-12);
    }

    #[test]
    fn river_share_counts_rivers_outside_lakes_over_all_cells() {
        let m = compute(&world(true, true), None);
        // cells 1-4 are river outside the lake: 4 / 16.
        assert!((num(&m, "rivers_pct") - 25.0).abs() < 1e-12);
    }

    #[test]
    fn the_chime_walk_comes_from_travel() {
        let w = world(true, true);
        let mut travel = Metrics::default();
        travel.0.insert("chime_to_chime_min".to_owned(), 12.34);
        let m = compute(&w, Some(&travel));
        assert!((num(&m, "travel.chime_to_chime_min") - 12.3).abs() < 1e-12);
        assert!(compute(&w, None).get("travel.chime_to_chime_min").is_none());
        assert!(
            compute(&w, Some(&Metrics::default()))
                .get("travel.chime_to_chime_min")
                .is_none()
        );
    }

    #[test]
    fn a_metric_without_its_input_is_omitted() {
        let m = compute(&world(false, false), None);
        assert!(m.get("vegetation_cover_pct").is_none());
        // no climate value: no precipitation; the height still gives its relief.
        assert!(m.get("precip_mm_yr.median").is_none());
        assert!(m.get("elevation_m.relief").is_some());
    }

    #[test]
    fn precipitation_falls_back_to_the_fields_of_the_height() {
        let mut w = world(false, true);
        let climate = Climate {
            params: ClimateParams::default(),
            fields: None,
        };
        w.insert("climate", Layer::Value(Box::new(climate)))
            .unwrap();
        let m = compute(&w, None);
        assert!(num(&m, "precip_mm_yr.median") >= 120.0);
    }
}
