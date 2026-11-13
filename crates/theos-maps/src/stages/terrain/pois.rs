//! The `pois` stage: ruins, Chimes and gates, ported from 0.3.0's `features.py::pois`.
//!
//! * `ruin`: the best cells of `max(prominence, 0) (0.4 + [hard rock]) [above the 55th height
//!   percentile] u`, at least 5 km apart, with the prominence `height - gauss(height)` and the
//!   noise `u = 1 + 0.15 r`; hard rock is granite, quartzite or basalt;
//! * `chime` (0.3.0's extraction point): the best cells of the base score `flat [below the 75th
//!   height percentile] [T > 0] [not river] [not lake] u` with `flat = exp(-gauss(slope) / 0.06)`,
//!   at least `min_chime_sep_km` apart and `chime_min_km_from_settlement` from every settlement;
//! * `gate-down` (0.3.0's arrival portal): the best cell of the base score where the prominence
//!   is positive, 2 km from every settlement;
//! * `gate-up` (0.3.0's ascent portal), with `gates = 2`: the best cell of `max(prominence, 0)
//!   flat` farther from the `gate-down` than 0.6 of the frame's diagonal, 2 km from every
//!   settlement.
//!
//! Every score is zero outside the frame's margin. A map without settlements applies no distance
//! to settlements. The features carry no properties (`docs/map-package.md` §5); 0.3.0's culture
//! and note texts are dropped.
//!
//! The lengths of 0.3.0 (cells of 62.5 m) are metres here: prominence sigma 375 m (6 cells), slope
//! smoothing sigma 250 m (4 cells). The gate pick's separation is one cell, so a pick only
//! clears itself.
//!
//! Draws from the stage's stream `placing`: one noise value per cell in index order, nothing
//! else. The metrics are the numbers of `ruin`, `chime`, `gate_down` and `gate_up`.

use serde::Deserialize;

use crate::features::{Feature, FeatureSet, Geometry, interior, pick};
use crate::grid::Grid;
use crate::kernels::KernelError;
use crate::kernels::filters::gaussian;
use crate::kernels::num::{max, to_f64};
use crate::kernels::stats::percentile;
use crate::rng::StageRng;
use crate::stage::{
    LayerSpec, MapKind, Metrics, ParamError, StageContext, StageError, StageReport, TypedStage,
};
use crate::stages::terrain::rocks::Rock;
use crate::world::{Layer, MapFrame, ValueType, World};
use std::collections::BTreeMap;

/// Sigma of the smoothing the prominence is measured against, 0.3.0's 6 cells of 62.5 m.
const PROMINENCE_SIGMA_M: f64 = 375.0;
/// Sigma of the slope smoothing, 0.3.0's 4 cells of 62.5 m.
const SLOPE_SIGMA_M: f64 = 250.0;
/// The least distance between two ruins, m.
const RUIN_SPACING_M: f64 = 5000.0;
/// The distance a gate keeps from every settlement, m.
const GATE_FROM_SETTLEMENT_M: f64 = 2000.0;
/// The share of the frame's diagonal between the two gates.
const FAR_SHARE: f64 = 0.6;
/// Ruins stand above this percentile of the heights.
const RUIN_HEIGHT_PERCENTILE: f64 = 55.0;
/// Chimes and the arrival lie below this percentile of the heights.
const LOW_HEIGHT_PERCENTILE: f64 = 75.0;
/// The metric names, in the order of the types the stage adds.
const METRICS: [&str; 4] = ["ruin", "chime", "gate_down", "gate_up"];

/// The parameters of `pois`.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(default)]
pub struct PoisParams {
    /// How many ruins.
    pub ruins: usize,
    /// How many Chimes.
    pub chimes: usize,
    /// How many gates: 0, 1 (only `gate-down`) or 2 (`gate-down` and `gate-up`).
    pub gates: u32,
    /// The least distance between two Chimes, km.
    pub min_chime_sep_km: f64,
    /// The least distance of a Chime from a settlement, km.
    pub chime_min_km_from_settlement: f64,
    /// The margin kept free, a share of the shorter side (spec ruling 46).
    pub edge_margin: f64,
}

impl Default for PoisParams {
    fn default() -> Self {
        Self {
            ruins: 2,
            chimes: 2,
            gates: 0,
            min_chime_sep_km: 8.0,
            chime_min_km_from_settlement: 3.0,
            edge_margin: 0.025,
        }
    }
}

/// The rasters the placing reads.
struct Inputs<'a> {
    height: &'a Grid<f64>,
    slope: &'a Grid<f64>,
    temperature: &'a Grid<f64>,
    rock: &'a Grid<u8>,
    river: &'a Grid<bool>,
    lake: &'a Grid<bool>,
}

/// A point of interest of type `ty` at `at`.
fn poi(ty: &str, at: [f64; 2]) -> Feature {
    Feature {
        kind: "poi",
        ty: ty.to_owned(),
        geometry: Geometry::Point(at),
        props: BTreeMap::new(),
    }
}

/// The scores the picks share.
struct Scores {
    /// The cells inside the frame's margin.
    inside: Grid<bool>,
    /// `height - gauss(height)`.
    prominence: Grid<f64>,
    /// `exp(-gauss(slope) / 0.06)`.
    flat: Grid<f64>,
    /// The base score of Chimes and the arrival.
    base: Grid<f64>,
    /// The ruin score.
    ruin: Grid<f64>,
}

/// Builds the scores; draws one noise value per cell.
fn scores(
    inputs: &Inputs<'_>,
    frame: &MapFrame,
    params: &PoisParams,
    rng: &mut StageRng,
) -> Result<Scores, KernelError> {
    let mut noise = Grid::filled(frame.rows, frame.cols, 1.0);
    for n in noise.as_mut_slice() {
        *n = 1.0 + 0.15 * rng.uniform();
    }
    let inside = interior(frame, params.edge_margin);
    let smooth = gaussian(inputs.height, PROMINENCE_SIGMA_M, frame.cell_m);
    let mut prominence = inputs.height.clone();
    for (p, s) in prominence.as_mut_slice().iter_mut().zip(smooth.as_slice()) {
        *p -= s;
    }
    let flat = gaussian(inputs.slope, SLOPE_SIGMA_M, frame.cell_m).map(|s| libm::exp(-s / 0.06));
    let high = percentile(inputs.height.as_slice(), RUIN_HEIGHT_PERCENTILE)?;
    let low = percentile(inputs.height.as_slice(), LOW_HEIGHT_PERCENTILE)?;
    let mut base = Grid::filled(frame.rows, frame.cols, 0.0);
    let mut ruin = Grid::filled(frame.rows, frame.cols, 0.0);
    for i in 0..base.len() {
        if !inside.as_slice()[i] {
            continue;
        }
        let z = inputs.height.as_slice()[i];
        let n = noise.as_slice()[i];
        if z > high {
            let hard = Rock::from_code(inputs.rock.as_slice()[i]).is_some_and(Rock::is_hard);
            let rock = 0.4 + if hard { 1.0 } else { 0.0 };
            ruin.as_mut_slice()[i] = max(prominence.as_slice()[i], 0.0) * rock * n;
        }
        let dry = !inputs.river.as_slice()[i] && !inputs.lake.as_slice()[i];
        if z < low && inputs.temperature.as_slice()[i] > 0.0 && dry {
            base.as_mut_slice()[i] = flat.as_slice()[i] * n;
        }
    }
    Ok(Scores {
        inside,
        prominence,
        flat,
        base,
        ruin,
    })
}

/// The gates: the arrival, and with `gates = 2` the ascent far from it.
fn gates(
    scores: &Scores,
    frame: &MapFrame,
    params: &PoisParams,
    settlements: &[[f64; 2]],
) -> Vec<Feature> {
    let mut out = Vec::new();
    if params.gates == 0 {
        return out;
    }
    let mut arrival = scores.base.clone();
    for (v, &p) in arrival
        .as_mut_slice()
        .iter_mut()
        .zip(scores.prominence.as_slice())
    {
        if p <= 0.0 {
            *v = 0.0;
        }
    }
    let one_cell = frame.cell_m;
    let down = pick(
        &arrival,
        1,
        one_cell,
        settlements,
        GATE_FROM_SETTLEMENT_M,
        frame,
    );
    let Some(&(row, col)) = down.first() else {
        return out;
    };
    let from = frame.centre_m(row, col);
    out.push(poi("gate-down", from));
    if params.gates < 2 {
        return out;
    }
    let [width, height] = frame.size_m();
    let far_m = FAR_SHARE * libm::sqrt(width * width + height * height);
    let mut ascent = Grid::filled(frame.rows, frame.cols, 0.0);
    for i in 0..ascent.len() {
        let (row, col) = ascent.cell(i);
        let [cx, cy] = frame.centre_m(row, col);
        let (dx, dy) = (cx - from[0], cy - from[1]);
        if scores.inside.as_slice()[i] && libm::sqrt(dx * dx + dy * dy) > far_m {
            ascent.as_mut_slice()[i] =
                max(scores.prominence.as_slice()[i], 0.0) * scores.flat.as_slice()[i];
        }
    }
    let up = pick(
        &ascent,
        1,
        one_cell,
        settlements,
        GATE_FROM_SETTLEMENT_M,
        frame,
    );
    if let Some(&(row, col)) = up.first() {
        out.push(poi("gate-up", frame.centre_m(row, col)));
    }
    out
}

/// Every point of interest to add, in the order ruins, Chimes, gates.
fn place(
    inputs: &Inputs<'_>,
    frame: &MapFrame,
    params: &PoisParams,
    settlements: &[[f64; 2]],
    rng: &mut StageRng,
) -> Result<Vec<Feature>, KernelError> {
    let s = scores(inputs, frame, params, rng)?;
    let mut out = Vec::new();
    for (r, c) in pick(&s.ruin, params.ruins, RUIN_SPACING_M, &[], 0.0, frame) {
        out.push(poi("ruin", frame.centre_m(r, c)));
    }
    let chimes = pick(
        &s.base,
        params.chimes,
        params.min_chime_sep_km * 1000.0,
        settlements,
        params.chime_min_km_from_settlement * 1000.0,
        frame,
    );
    for (r, c) in chimes {
        out.push(poi("chime", frame.centre_m(r, c)));
    }
    out.extend(gates(&s, frame, params, settlements));
    Ok(out)
}

/// The stage `pois`.
pub struct Pois;

impl TypedStage for Pois {
    type Params = PoisParams;
    const ID: &'static str = "pois";
    const SUMMARY: &'static str =
        "Ruins on prominent hard ridges, Chimes on flat dry ground, and the gates.";
    const KINDS: &'static [MapKind] = &[MapKind::Terrain];

    fn reads(&self, _: &PoisParams) -> Vec<LayerSpec> {
        vec![
            LayerSpec::f64("height"),
            LayerSpec::f64("slope"),
            LayerSpec::f64("temperature"),
            LayerSpec::u8("rock"),
            LayerSpec::mask("river"),
            LayerSpec::mask("lake"),
            LayerSpec::value("features", FeatureSet::NAME).optional(),
        ]
    }

    fn writes(&self, _: &PoisParams) -> Vec<LayerSpec> {
        vec![LayerSpec::value("features", FeatureSet::NAME)]
    }

    fn metric_names(&self, _: &PoisParams) -> Vec<&'static str> {
        METRICS.to_vec()
    }

    fn validate(&self, params: &mut PoisParams) -> Result<(), ParamError> {
        let at = |name: &str, message: &str| ParamError::at(Self::ID, name, message);
        if params.gates > 2 {
            return Err(at("gates", "must be 0, 1 or 2"));
        }
        let sep = params.min_chime_sep_km;
        if !(sep.is_finite() && sep > 0.0) {
            return Err(at("min_chime_sep_km", "must be positive and finite"));
        }
        let far = params.chime_min_km_from_settlement;
        if !(far.is_finite() && far >= 0.0) {
            return Err(at(
                "chime_min_km_from_settlement",
                "must be at least 0 and finite",
            ));
        }
        let m = params.edge_margin;
        if !(m.is_finite() && (0.0..0.5).contains(&m)) {
            return Err(at("edge_margin", "must be at least 0 and below 0.5"));
        }
        Ok(())
    }

    fn run(
        &self,
        ctx: &mut StageContext<'_>,
        params: &PoisParams,
    ) -> Result<StageReport, StageError> {
        let frame = *ctx.frame();
        let mut rng = ctx.sub_rng("placing");
        let settlements = ctx
            .value_opt::<FeatureSet>("features")?
            .map_or_else(Vec::new, |set| set.points("settlement", None));
        let found = {
            let inputs = Inputs {
                height: ctx.f64("height")?,
                slope: ctx.f64("slope")?,
                temperature: ctx.f64("temperature")?,
                rock: ctx.u8("rock")?,
                river: ctx.mask("river")?,
                lake: ctx.mask("lake")?,
            };
            place(&inputs, &frame, params, &settlements, &mut rng)
                .map_err(|e| ctx.fail(e.to_string()))?
        };
        let mut report = StageReport::default();
        for (name, ty) in METRICS
            .into_iter()
            .zip(["ruin", "chime", "gate-down", "gate-up"])
        {
            let n = to_f64(found.iter().filter(|f| f.ty == ty).count());
            report.values.insert(name.to_owned(), n);
        }
        let mut set = if ctx.value_opt::<FeatureSet>("features")?.is_some() {
            *ctx.take_value::<FeatureSet>("features")?
        } else {
            FeatureSet::default()
        };
        for f in found {
            set.add(f);
        }
        ctx.write("features", Layer::Value(Box::new(set)))?;
        Ok(report)
    }

    fn metrics(&self, _: &World, _: &PoisParams, report: &StageReport) -> Metrics {
        Metrics(report.values.clone())
    }
}

#[cfg(test)]
mod tests {
    use crate::features::{Feature, FeatureSet, Geometry};
    use crate::grid::Grid;
    use crate::kernels::num::to_f64;
    use crate::stage::Stage;
    use crate::stages::terrain::pois::{Pois, PoisParams};
    use crate::stages::terrain::support::{run_stage, table, world};
    use crate::world::{Layer, World};

    const N: usize = 64;
    const GRANITE: u8 = 0;
    const SHALE: u8 = 5;

    fn bump(r: usize, c: usize, centre: (f64, f64)) -> f64 {
        let (dr, dc) = (to_f64(r) - centre.0, to_f64(c) - centre.1);
        300.0 * libm::exp(-(dr * dr + dc * dc) / 32.0)
    }

    /// A ramp over the columns with ripples, a granite hill at (16, 20) and a shale hill of the
    /// same shape at (48, 20); flat ground only in columns 0 to 9.
    fn setup() -> World {
        let mut w = world(N, N);
        let height = (0..N * N)
            .map(|i| {
                let (r, c) = (i / N, i % N);
                let ripple = 4.0 * libm::sin(0.9 * to_f64(r)) * libm::sin(0.7 * to_f64(c));
                100.0
                    + 0.5 * to_f64(c)
                    + ripple
                    + bump(r, c, (16.0, 20.0))
                    + bump(r, c, (48.0, 20.0))
            })
            .collect();
        w.insert("height", Layer::F64(Grid::from_vec(N, N, height).unwrap()))
            .unwrap();
        let slope = (0..N * N)
            .map(|i| if i % N < 10 { 0.02 } else { 0.1 })
            .collect();
        w.insert("slope", Layer::F64(Grid::from_vec(N, N, slope).unwrap()))
            .unwrap();
        w.insert("temperature", Layer::F64(Grid::filled(N, N, 10.0)))
            .unwrap();
        let rock = (0..N * N)
            .map(|i| if i / N < 32 { GRANITE } else { SHALE })
            .collect();
        w.insert("rock", Layer::U8(Grid::from_vec(N, N, rock).unwrap()))
            .unwrap();
        w.insert("river", Layer::Mask(Grid::filled(N, N, false)))
            .unwrap();
        w.insert("lake", Layer::Mask(Grid::filled(N, N, false)))
            .unwrap();
        w
    }

    fn with_settlement(w: &mut World, at: [f64; 2]) {
        let mut set = FeatureSet::default();
        set.add(Feature {
            kind: "settlement",
            ty: "hamlet".into(),
            geometry: Geometry::Point(at),
            props: std::collections::BTreeMap::new(),
        });
        w.insert("features", Layer::Value(Box::new(set))).unwrap();
    }

    fn points(w: &World, ty: &str) -> Vec<[f64; 2]> {
        w.value::<FeatureSet>("features")
            .unwrap()
            .of_kind("poi")
            .filter(|(_, f)| f.ty == ty)
            .map(|(_, f)| match f.geometry {
                Geometry::Point(p) => p,
                _ => panic!("a poi is a point"),
            })
            .collect()
    }

    fn dist(a: [f64; 2], b: [f64; 2]) -> f64 {
        libm::sqrt((a[0] - b[0]) * (a[0] - b[0]) + (a[1] - b[1]) * (a[1] - b[1]))
    }

    #[test]
    fn ruins_sit_on_prominent_hard_ridges() {
        let mut w = setup();
        run_stage(&mut w, &Pois, "ruins = 1\nchimes = 0").unwrap();
        let ruins = points(&w, "ruin");
        assert_eq!(ruins.len(), 1);
        // The granite hill summit, (16, 20): both hills are as prominent, hard rock decides.
        let summit = [2050.0, 1650.0];
        assert!(dist(ruins[0], summit) < 500.0, "{:?}", ruins[0]);
    }

    /// With the cells farther from the arrival than 0.6 of the diagonal levelled at 0 m (a step
    /// down has no positive prominence on its low side), the ascent has no cell and only the
    /// arrival is placed.
    #[test]
    fn the_ascent_needs_a_far_cell_with_prominence() {
        let mut first = setup();
        run_stage(
            &mut first,
            &Pois,
            "ruins = 0
chimes = 0
gates = 1",
        )
        .unwrap();
        let arrival = points(&first, "gate-down")[0];
        let far = 0.6 * dist([0.0, 0.0], [6400.0, 6400.0]);
        let mut w = setup();
        let mut height = w.f64("height").unwrap().clone();
        for i in 0..N * N {
            let at = [to_f64(i % N) * 100.0 + 50.0, to_f64(i / N) * 100.0 + 50.0];
            if dist(at, arrival) > far {
                height.as_mut_slice()[i] = 0.0;
            }
        }
        w.insert("height", Layer::F64(height)).unwrap();
        run_stage(
            &mut w,
            &Pois,
            "ruins = 0
chimes = 0
gates = 2",
        )
        .unwrap();
        assert_eq!(points(&w, "gate-down"), [arrival]);
        assert_eq!(points(&w, "gate-up").len(), 0);
    }

    #[test]
    fn ruins_keep_their_spacing() {
        let mut w = setup();
        run_stage(
            &mut w,
            &Pois,
            "ruins = 4
chimes = 0",
        )
        .unwrap();
        let ruins = points(&w, "ruin");
        assert_ne!(ruins.len(), 0);
        for (i, a) in ruins.iter().enumerate() {
            for b in &ruins[i + 1..] {
                assert!(dist(*a, *b) >= 5000.0, "{a:?} {b:?}");
            }
        }
    }

    /// A plateau of shale over the columns from 30 with a granite hill beside it, lower than the
    /// plateau but far more prominent than anything on it.
    #[test]
    fn ruins_stand_above_the_55th_height_percentile() {
        let mut w = setup();
        let height = (0..N * N)
            .map(|i| {
                let (r, c) = (i / N, i % N);
                if c >= 30 {
                    600.0 + 0.01 * to_f64(c)
                } else {
                    100.0
                        + 450.0
                            * libm::exp(
                                -(libm::pow(to_f64(r) - 32.0, 2.0)
                                    + libm::pow(to_f64(c) - 10.0, 2.0))
                                    / 32.0,
                            )
                }
            })
            .collect();
        w.insert("height", Layer::F64(Grid::from_vec(N, N, height).unwrap()))
            .unwrap();
        let rock = (0..N * N)
            .map(|i| if i % N < 30 { GRANITE } else { SHALE })
            .collect();
        w.insert("rock", Layer::U8(Grid::from_vec(N, N, rock).unwrap()))
            .unwrap();
        run_stage(
            &mut w,
            &Pois,
            "ruins = 1
chimes = 0",
        )
        .unwrap();
        let ruins = points(&w, "ruin");
        assert_eq!(ruins.len(), 1);
        // The hill peaks at 550 m, below the 55th percentile of the heights (on the plateau).
        assert!(ruins[0][0] >= 3000.0, "{:?}", ruins[0]);
    }

    /// Flat ground only on the high columns; the heights rise by 5 m per column.
    #[test]
    fn chimes_keep_to_the_low_ground() {
        let mut w = setup();
        let height = (0..N * N)
            .map(|i| {
                let (r, c) = (i / N, i % N);
                100.0 + 5.0 * to_f64(c) + libm::sin(0.9 * to_f64(r))
            })
            .collect();
        w.insert("height", Layer::F64(Grid::from_vec(N, N, height).unwrap()))
            .unwrap();
        let slope = (0..N * N)
            .map(|i| if i % N >= 50 { 0.0 } else { 0.1 })
            .collect();
        w.insert("slope", Layer::F64(Grid::from_vec(N, N, slope).unwrap()))
            .unwrap();
        run_stage(
            &mut w,
            &Pois,
            "ruins = 0
chimes = 2
min_chime_sep_km = 1.0",
        )
        .unwrap();
        let chimes = points(&w, "chime");
        assert_eq!(chimes.len(), 2);
        // The 75th height percentile lies at column 48.
        for c in chimes {
            assert!(c[0] < 4900.0, "{c:?}");
        }
    }

    #[test]
    fn chimes_keep_their_separation_and_distance_from_settlements() {
        let mut w = setup();
        let hamlet = [550.0, 3250.0];
        with_settlement(&mut w, hamlet);
        let params =
            "ruins = 0\nchimes = 3\nmin_chime_sep_km = 1.5\nchime_min_km_from_settlement = 1.0";
        run_stage(&mut w, &Pois, params).unwrap();
        let chimes = points(&w, "chime");
        assert_eq!(chimes.len(), 3);
        for (i, a) in chimes.iter().enumerate() {
            assert!(dist(*a, hamlet) >= 1000.0, "{a:?}");
            for b in &chimes[i + 1..] {
                assert!(dist(*a, *b) >= 1500.0, "{a:?} {b:?}");
            }
        }
    }

    #[test]
    fn gates_are_placed_as_asked() {
        let run = |gates: u32| {
            let mut w = setup();
            let params = format!("ruins = 0\nchimes = 0\ngates = {gates}");
            run_stage(&mut w, &Pois, &params).unwrap();
            w
        };
        let none = run(0);
        assert!(points(&none, "gate-down").is_empty() && points(&none, "gate-up").is_empty());
        let one = run(1);
        assert_eq!(points(&one, "gate-down").len(), 1);
        assert_eq!(points(&one, "gate-up").len(), 0);
        let two = run(2);
        let (down, up) = (points(&two, "gate-down"), points(&two, "gate-up"));
        assert_eq!((down.len(), up.len()), (1, 1));
        let diagonal = dist([0.0, 0.0], [6400.0, 6400.0]);
        assert!(dist(down[0], up[0]) > 0.6 * diagonal, "{down:?} {up:?}");
        // The arrival lies on the flat ground of the first columns.
        assert!(down[0][0] < 1000.0, "{down:?}");
        let features = two.value::<FeatureSet>("features").unwrap();
        assert_eq!(features.len(), 2);
        assert!(features.iter().all(|(_, f)| f.props.is_empty()));
    }

    #[test]
    fn without_settlements_no_distance_applies() {
        let params =
            "ruins = 0\nchimes = 2\nmin_chime_sep_km = 0.5\nchime_min_km_from_settlement = 100.0";
        let mut bare = setup();
        run_stage(&mut bare, &Pois, params).unwrap();
        assert_eq!(points(&bare, "chime").len(), 2);
        let mut settled = setup();
        with_settlement(&mut settled, [3200.0, 3200.0]);
        run_stage(&mut settled, &Pois, params).unwrap();
        assert_eq!(points(&settled, "chime").len(), 0);
    }

    #[test]
    fn places_stay_off_the_margin() {
        let mut w = setup();
        run_stage(
            &mut w,
            &Pois,
            "ruins = 2\nchimes = 3\ngates = 2\nedge_margin = 0.1\nmin_chime_sep_km = 1.0",
        )
        .unwrap();
        let set = w.value::<FeatureSet>("features").unwrap();
        assert!(set.len() >= 5);
        for (_, f) in set.iter() {
            let Geometry::Point([x, y]) = f.geometry else {
                panic!("a poi is a point")
            };
            // 0.1 x 64 = 6.4, rounded to 6 cells of 100 m.
            for v in [x, y] {
                assert!((600.0..5800.0).contains(&v), "{:?} {v}", f.ty);
            }
        }
    }

    #[test]
    fn the_same_inputs_give_the_same_pois() {
        let (mut a, mut b) = (setup(), setup());
        run_stage(&mut a, &Pois, "gates = 2").unwrap();
        run_stage(&mut b, &Pois, "gates = 2").unwrap();
        assert_eq!(
            a.value::<FeatureSet>("features"),
            b.value::<FeatureSet>("features")
        );
    }

    #[test]
    fn existing_features_stay() {
        let mut w = setup();
        with_settlement(&mut w, [3200.0, 3200.0]);
        run_stage(&mut w, &Pois, "ruins = 1\nchimes = 1").unwrap();
        let set = w.value::<FeatureSet>("features").unwrap();
        assert_eq!(set.len(), 3);
        assert_eq!(
            set.iter().next().map(|(_, f)| f.ty.as_str()),
            Some("hamlet")
        );
    }

    #[test]
    fn parameters_have_defaults_and_are_checked() {
        let d = PoisParams::default();
        assert_eq!((d.ruins, d.chimes, d.gates), (2, 2, 0));
        assert!((d.min_chime_sep_km - 8.0).abs() < 1e-12);
        assert!((d.chime_min_km_from_settlement - 3.0).abs() < 1e-12);
        assert!((d.edge_margin - 0.025).abs() < 1e-12);
        assert!(Pois.params(&table("")).is_ok());
        for bad in [
            "gates = 3",
            "min_chime_sep_km = 0.0",
            "min_chime_sep_km = nan",
            "chime_min_km_from_settlement = -1.0",
            "edge_margin = 0.5",
            "edge_margin = -0.1",
        ] {
            assert!(Pois.params(&table(bad)).is_err(), "{bad}");
        }
        let err = Pois.params(&table("gates = 5")).unwrap_err();
        assert_eq!(err.parameter.as_deref(), Some("gates"));
        let err = Pois.params(&table("extractions = 2")).unwrap_err();
        assert!(err.to_string().contains("extractions"), "{err}");
    }

    #[test]
    fn the_stage_reports_its_counts_under_declared_names() {
        let mut w = setup();
        let params = "ruins = 1\nchimes = 2\ngates = 2\nmin_chime_sep_km = 1.0";
        let report = run_stage(&mut w, &Pois, params).unwrap();
        let defaults = Pois.params(&table("")).unwrap();
        let metrics = Stage::metrics(&Pois, &w, defaults.as_ref(), &report);
        let mut names = Stage::metric_names(&Pois, defaults.as_ref());
        names.sort_unstable();
        let keys: Vec<&str> = metrics.0.keys().map(String::as_str).collect();
        assert_eq!(names, keys);
        assert_eq!(metrics.0.get("ruin"), Some(&1.0));
        assert_eq!(metrics.0.get("chime"), Some(&2.0));
        assert_eq!(metrics.0.get("gate_down"), Some(&1.0));
        assert_eq!(metrics.0.get("gate_up"), Some(&1.0));
    }
}
