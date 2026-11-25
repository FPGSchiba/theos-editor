//! The `settlements` stage: sites for cities, towns, villages and hamlets, ported from 0.3.0's
//! `features.py::settlements`.
//!
//! The site quality of a cell is flat ground, water close but not underfoot, river confluences
//! and lake shores, soft-rock farmland and warmth, zero on lakes, big rivers, the frame's margin
//! and where the temperature is 3 degrees or less, times a noise factor `1 + 0.1 u`:
//!
//! ```text
//! flat * exp(-((d - 0.2) / 0.45)^2) * (0.6 + 0.4 confluence + 0.3 shore) * (0.7 + 0.3 soil)
//! ```
//!
//! with `d` the distance to river or lake in km and `flat = exp(-gauss(slope) / 0.06)`. The
//! hinterland is the quality's mean over a 3 km window. Each class in turn (city first) takes
//! the best cells of `quality * (hinterland / its maximum)^e`, `e` 1.5 for cities and towns and
//! 0.3 otherwise, at least its own spacing from each other and 0.6 spacings from every settlement
//! placed before.
//!
//! The lengths of 0.3.0 (cells of 62.5 m) are metres here: slope smoothing 187.5 m, confluence
//! window 1.2 km, shore 0.5 km, hinterland window 3 km.
//!
//! Draws from the stage's stream `siting`: first one per cell in index order (the noise), then
//! per settlement in pick order its name (see [`generated_name`]) and its population factor.
//! The metrics are the number of settlements, `settlements`, and of each class.

use serde::{Deserialize, Serialize};

use crate::features::{Feature, FeatureSet, Prop, generated_name, interior, pick};
use crate::grid::Grid;
use crate::kernels::KernelError;
use crate::kernels::filters::{gaussian, maximum, uniform};
use crate::kernels::morphology::{Structure, dilate};
use crate::kernels::num::{max, round_to, to_f64, to_index};
use crate::rng::StageRng;
use crate::stage::{
    Check, LayerSpec, MapKind, ParamError, StageContext, StageError, StageReport, TypedStage,
};
use crate::stages::terrain::rocks::Rock;
use crate::stages::terrain::{add_features, water_distance_m};
use crate::world::{Layer, MapFrame, ValueType};
use std::collections::BTreeMap;
use std::collections::BTreeSet;

/// Sigma of the slope smoothing, 0.3.0's 3 cells of 62.5 m.
const SLOPE_SIGMA_M: f64 = 187.5;
/// The window of the confluence maximum, m.
const CONFLUENCE_M: f64 = 1200.0;
/// How far a lake's shore reaches, m.
const SHORE_M: f64 = 500.0;
/// The window of the hinterland mean, m.
const HINTERLAND_M: f64 = 3000.0;
/// The nearest a river or well counts as a river settlement, km.
const RIVER_KM: f64 = 0.6;
/// The share of a class's spacing kept from earlier settlements.
const TAKEN_SHARE: f64 = 0.6;
/// Temperature above which people settle, degrees Celsius.
const WARM_C: f64 = 3.0;

/// One settlement class.
struct Class {
    ty: &'static str,
    population: f64,
    exponent: f64,
}

/// The classes in the order they take sites.
const CLASSES: [Class; 4] = [
    Class {
        ty: "city",
        population: 9000.0,
        exponent: 1.5,
    },
    Class {
        ty: "town",
        population: 2500.0,
        exponent: 1.5,
    },
    Class {
        ty: "village",
        population: 400.0,
        exponent: 0.3,
    },
    Class {
        ty: "hamlet",
        population: 60.0,
        exponent: 0.3,
    },
];

/// How many settlements of each class. A `counts` table in a recipe replaces the default as a
/// whole: a class it leaves out gets none, as in 0.3.0.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
pub struct Counts {
    /// Cities.
    #[serde(default)]
    pub city: usize,
    /// Towns.
    #[serde(default)]
    pub town: usize,
    /// Villages.
    #[serde(default)]
    pub village: usize,
    /// Hamlets.
    #[serde(default)]
    pub hamlet: usize,
}

impl Counts {
    /// The count of class `ty`.
    fn of(&self, ty: &str) -> usize {
        match ty {
            "city" => self.city,
            "town" => self.town,
            "village" => self.village,
            _ => self.hamlet,
        }
    }
}

impl Default for Counts {
    fn default() -> Self {
        Self {
            city: 0,
            town: 2,
            village: 4,
            hamlet: 4,
        }
    }
}

/// The spacing of each class in km: the least distance between two settlements of the class.
#[derive(Debug, Clone, Copy, PartialEq, Deserialize, Serialize)]
#[serde(default)]
pub struct SpacingKm {
    /// Cities.
    pub city: f64,
    /// Towns.
    pub town: f64,
    /// Villages.
    pub village: f64,
    /// Hamlets.
    pub hamlet: f64,
}

impl SpacingKm {
    /// The spacing of class `ty`.
    fn of(&self, ty: &str) -> f64 {
        match ty {
            "city" => self.city,
            "town" => self.town,
            "village" => self.village,
            _ => self.hamlet,
        }
    }
}

impl Default for SpacingKm {
    fn default() -> Self {
        Self {
            city: 7.0,
            town: 4.5,
            village: 2.5,
            hamlet: 1.4,
        }
    }
}

/// The parameters of `settlements`.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(default)]
pub struct SettlementsParams {
    /// How many settlements of each class; default 0.3.0's two towns, four villages and four
    /// hamlets.
    pub counts: Counts,
    /// The spacing of each class, km.
    pub spacing_km: SpacingKm,
    /// The margin kept free of settlements, a share of the shorter side (spec ruling 46).
    pub edge_margin: f64,
    /// Names to draw from before the syllable lists; none by default.
    pub names: Option<Vec<String>>,
}

impl Default for SettlementsParams {
    fn default() -> Self {
        Self {
            counts: Counts::default(),
            spacing_km: SpacingKm::default(),
            edge_margin: 0.025,
            names: None,
        }
    }
}

/// The metric names, in the order of [`CLASSES`] after `settlements`.
const METRICS: [&str; 5] = ["settlements", "city", "town", "village", "hamlet"];

/// The rasters the siting reads.
struct Inputs<'a> {
    height: &'a Grid<f64>,
    slope: &'a Grid<f64>,
    temperature: &'a Grid<f64>,
    river: &'a Grid<bool>,
    lake: &'a Grid<bool>,
    big_river: &'a Grid<bool>,
    discharge: &'a Grid<f64>,
    rock: &'a Grid<u8>,
}

/// Whether `code` is farmland: alluvium, shale, sandstone or limestone.
fn is_soil(code: u8) -> bool {
    matches!(
        Rock::from_code(code),
        Some(Rock::Alluvium | Rock::Shale | Rock::Sandstone | Rock::Limestone)
    )
}

/// The distance of every cell to the nearest river or lake cell, km; infinite when there is none.
fn water_distance_km(inputs: &Inputs<'_>, frame: &MapFrame) -> Result<Grid<f64>, KernelError> {
    let dist = water_distance_m(inputs.river, inputs.lake, frame.cell_m)?;
    Ok(dist.map_or_else(
        || Grid::filled(frame.rows, frame.cols, f64::INFINITY),
        |d| d.map(|m| m / 1000.0),
    ))
}

/// The site quality before the noise: see the module docs.
fn base_quality(
    inputs: &Inputs<'_>,
    frame: &MapFrame,
    d_km: &Grid<f64>,
    shore: &Grid<bool>,
    edge_margin: f64,
) -> Grid<f64> {
    let flat = gaussian(inputs.slope, SLOPE_SIGMA_M, frame.cell_m);
    let log_q = inputs.discharge.map(libm::log1p);
    let top = max(
        log_q
            .as_slice()
            .iter()
            .fold(f64::NEG_INFINITY, |m, &v| max(m, v)),
        1e-6,
    );
    let mut on_river = log_q.clone();
    for (v, &r) in on_river
        .as_mut_slice()
        .iter_mut()
        .zip(inputs.river.as_slice())
    {
        if !r {
            *v = 0.0;
        }
    }
    let window = frame.cells_of_m(CONFLUENCE_M);
    let confluence = maximum(&on_river, window);
    let inside = interior(frame, edge_margin);
    let mut out = Grid::filled(frame.rows, frame.cols, 0.0);
    for (i, q) in out.as_mut_slice().iter_mut().enumerate() {
        let blocked = inputs.lake.as_slice()[i] || inputs.big_river.as_slice()[i];
        let warm = inputs.temperature.as_slice()[i] > WARM_C;
        if blocked || !warm || !inside.as_slice()[i] {
            continue;
        }
        let x = (d_km.as_slice()[i] - 0.2) / 0.45;
        let near_water = libm::exp(-(x * x));
        let shore_bonus = if shore.as_slice()[i] { 0.3 } else { 0.0 };
        let soil = if is_soil(inputs.rock.as_slice()[i]) {
            1.0
        } else {
            0.0
        };
        *q = libm::exp(-flat.as_slice()[i] / 0.06)
            * near_water
            * (0.6 + 0.4 * confluence.as_slice()[i] / top + shore_bonus)
            * (0.7 + 0.3 * soil);
    }
    out
}

/// The population of a settlement of `class`, truncated as 0.3.0's `int()`.
fn population(class: &Class, u: f64) -> i64 {
    let n = to_index(class.population * (0.7 + 0.6 * u), usize::MAX);
    i64::try_from(n).unwrap_or(i64::MAX)
}

/// The settlements to add, in the order they are picked, and the site quality.
///
/// The counts are a recipe's, and a name drawn from a recipe's list costs a pass over it, so
/// `cancel` is called while picking and before every settlement.
///
/// # Errors
/// [`KernelError`] from the distance and filter kernels; [`KernelError::Cancelled`] when
/// `cancel` returns `true`.
fn site(
    inputs: &Inputs<'_>,
    frame: &MapFrame,
    params: &SettlementsParams,
    rng: &mut StageRng,
    cancel: &dyn Fn() -> bool,
) -> Result<(Vec<Feature>, Grid<f64>), KernelError> {
    let d_km = water_distance_km(inputs, frame)?;
    let shore = dilate(
        inputs.lake,
        Structure::Cross,
        frame.cell_m,
        frame.cells_of_m(SHORE_M),
    );
    let mut quality = base_quality(inputs, frame, &d_km, &shore, params.edge_margin);
    for q in quality.as_mut_slice() {
        *q *= 1.0 + 0.1 * rng.uniform();
    }
    let window = frame.cells_of_m(HINTERLAND_M);
    let catchment = uniform(&quality, window).map(|v| max(v, 0.0));
    let catch_max = max(
        catchment.as_slice().iter().fold(0.0, |m, &v| max(m, v)),
        1e-9,
    );
    let names = params.names.as_deref();
    let mut used = BTreeSet::new();
    let mut taken: Vec<[f64; 2]> = Vec::new();
    let mut features = Vec::new();
    for class in &CLASSES {
        let n = params.counts.of(class.ty);
        if n == 0 {
            continue;
        }
        let spacing_m = params.spacing_km.of(class.ty) * 1000.0;
        let mut ranked = quality.clone();
        for (cell, &hinter) in ranked.as_mut_slice().iter_mut().zip(catchment.as_slice()) {
            *cell *= libm::pow(hinter / catch_max, class.exponent);
        }
        let picks = pick(
            &ranked,
            n,
            spacing_m,
            &taken,
            TAKEN_SHARE * spacing_m,
            frame,
            cancel,
        )?;
        for (r, c) in picks {
            if cancel() {
                return Err(KernelError::Cancelled);
            }
            let at = frame.centre_m(r, c);
            let name = generated_name(rng, &mut used, names);
            // Syllable names are drawn only once every list name is in `used`, so a generated
            // name never equals a list entry and this test tells the two apart exactly.
            let from_list = names.is_some_and(|l| l.contains(&name));
            let i = quality.index(r, c);
            let water = if shore.as_slice()[i] {
                "lake"
            } else if d_km.as_slice()[i] < RIVER_KM {
                "river"
            } else {
                "well"
            };
            let mut props = BTreeMap::new();
            props.insert("name".to_owned(), Prop::Text(name));
            if !from_list {
                props.insert("generated_name".to_owned(), Prop::Bool(true));
            }
            let pop = population(class, rng.uniform());
            props.insert("population".to_owned(), Prop::Int(pop));
            props.insert("water".to_owned(), Prop::Text(water.to_owned()));
            let metres = round_to(inputs.height.as_slice()[i], 10.0);
            props.insert("elevation_m".to_owned(), Prop::Num(metres));
            features.push(Feature::point("settlement", class.ty, at, props));
            taken.push(at);
        }
    }
    Ok((features, quality))
}

/// The stage `settlements`.
pub struct Settlements;

impl TypedStage for Settlements {
    type Params = SettlementsParams;
    const ID: &'static str = "settlements";
    const SUMMARY: &'static str =
        "Settlements by site quality: flat ground near water, cities on the best sites first.";
    const KINDS: &'static [MapKind] = &[MapKind::Terrain];

    fn reads(&self, _: &SettlementsParams) -> Vec<LayerSpec> {
        vec![
            LayerSpec::f64("height"),
            LayerSpec::f64("slope"),
            LayerSpec::f64("temperature"),
            LayerSpec::mask("river"),
            LayerSpec::mask("lake"),
            LayerSpec::mask("big_river"),
            LayerSpec::f64("discharge"),
            LayerSpec::u8("rock"),
            LayerSpec::value("features", FeatureSet::NAME).optional(),
        ]
    }

    fn writes(&self, _: &SettlementsParams) -> Vec<LayerSpec> {
        vec![
            LayerSpec::value("features", FeatureSet::NAME),
            LayerSpec::f64("site_quality"),
        ]
    }

    fn metric_names(&self, _: &SettlementsParams) -> Vec<&'static str> {
        METRICS.to_vec()
    }

    fn validate(&self, params: &mut SettlementsParams) -> Result<(), ParamError> {
        let check = Check::of(Self::ID);
        let s = &params.spacing_km;
        for (name, v) in [
            ("spacing_km.city", s.city),
            ("spacing_km.town", s.town),
            ("spacing_km.village", s.village),
            ("spacing_km.hamlet", s.hamlet),
        ] {
            check.positive(name, v)?;
        }
        check.in_closed_open("edge_margin", params.edge_margin, 0.0, 0.5)?;
        if params.names.iter().flatten().any(|n| n.trim().is_empty()) {
            return Err(ParamError::at(
                Self::ID,
                "names",
                "must not hold an empty name",
            ));
        }
        Ok(())
    }

    fn run(
        &self,
        ctx: &mut StageContext<'_>,
        params: &SettlementsParams,
    ) -> Result<StageReport, StageError> {
        let frame = *ctx.frame();
        let mut rng = ctx.sub_rng("siting");
        let (found, quality) = {
            let inputs = Inputs {
                height: ctx.f64("height")?,
                slope: ctx.f64("slope")?,
                temperature: ctx.f64("temperature")?,
                river: ctx.mask("river")?,
                lake: ctx.mask("lake")?,
                big_river: ctx.mask("big_river")?,
                discharge: ctx.f64("discharge")?,
                rock: ctx.u8("rock")?,
            };
            site(&inputs, &frame, params, &mut rng, &|| ctx.is_cancelled())
                .map_err(|e| ctx.kernel_failed(e))?
        };
        let mut report = StageReport::default();
        let mut total = 0.0;
        for class in &CLASSES {
            let n = to_f64(Feature::count_of(&found, class.ty));
            report.put(class.ty, n);
            total += n;
        }
        report.put("settlements", total);
        add_features(ctx, found)?;
        ctx.write("site_quality", Layer::F64(quality))?;
        Ok(report)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::features::{Feature, Geometry, Prop};
    use crate::kernels::num::to_f64;
    use crate::stage::{Stage, StageError};
    use crate::stages::terrain::support::{run_cancelled, run_stage, table, world};
    use crate::world::{Layer, World};

    const N: usize = 64;

    fn masked(f: impl Fn(usize, usize) -> bool) -> Layer {
        Layer::Mask(Grid::from_vec(N, N, (0..N * N).map(|i| f(i / N, i % N)).collect()).unwrap())
    }

    fn flat(v: f64) -> Layer {
        Layer::F64(Grid::filled(N, N, v))
    }

    /// A warm plain on alluvium with a river along column 3, a lake, a big river along column 50.
    fn setup() -> World {
        let mut w = world(N, N);
        let height = (0..N * N).map(|i| 100.0 + 0.25 * to_f64(i % N)).collect();
        w.insert("height", Layer::F64(Grid::from_vec(N, N, height).unwrap()))
            .unwrap();
        w.insert("slope", flat(0.02)).unwrap();
        w.insert("temperature", flat(12.0)).unwrap();
        w.insert("river", masked(|_, c| c == 3 || c == 50)).unwrap();
        let in_lake = |r: usize, c: usize| (30..36).contains(&r) && (40..46).contains(&c);
        w.insert("lake", masked(in_lake)).unwrap();
        w.insert("big_river", masked(|_, c| c == 50)).unwrap();
        w.insert("discharge", flat(2.0)).unwrap();
        w.insert("rock", Layer::U8(Grid::filled(N, N, 6))).unwrap();
        w
    }

    fn cell_of(w: &World, f: &Feature) -> (usize, usize) {
        let Geometry::Point([x, y]) = f.geometry else {
            panic!("a settlement is a point")
        };
        w.frame().cell_at_m(x, y)
    }

    fn settlements_of(w: &World) -> Vec<Feature> {
        w.value::<FeatureSet>("features")
            .unwrap()
            .of_kind("settlement")
            .map(|(_, f)| f.clone())
            .collect()
    }

    const SMALL: &str = "spacing_km = { city = 2.0, town = 1.0, village = 0.6, hamlet = 0.3 }\n";

    fn run(w: &mut World, extra: &str) -> crate::stage::StageReport {
        run_stage(w, &Settlements, &format!("{SMALL}{extra}")).unwrap()
    }

    #[test]
    fn settlements_avoid_lakes_big_rivers_and_the_margin() {
        let mut w = setup();
        run(
            &mut w,
            "counts = { town = 3, village = 5, hamlet = 5 }\nedge_margin = 0.05\n",
        );
        let found = settlements_of(&w);
        assert_eq!(found.len(), 13);
        let lake = w.mask("lake").unwrap().clone();
        let big = w.mask("big_river").unwrap().clone();
        for f in &found {
            let (r, c) = cell_of(&w, f);
            assert!(!lake.get(r, c) && !big.get(r, c), "{r},{c}");
            // 0.05 * 64 = 3.2, rounded to 3 cells.
            assert!(
                (3..N - 3).contains(&r) && (3..N - 3).contains(&c),
                "{r},{c}"
            );
        }
        let quality = w.f64("site_quality").unwrap();
        assert!(quality.get(32, 42) == 0.0 && quality.get(10, 50) == 0.0);
        assert!(quality.get(0, 10) == 0.0 && quality.get(10, 63) == 0.0);
        assert!(quality.get(10, 5) > 0.0);
        // The river on column 3 lies at the margin (3 cells): the cells beside it are open.
        assert!(quality.get(10, 2) == 0.0 && quality.get(10, 3) > 0.0);
    }

    #[test]
    fn cities_take_the_best_sites_first() {
        let only = |counts: &str| {
            let mut w = setup();
            run(&mut w, counts);
            settlements_of(&w)
                .iter()
                .map(|f| (f.ty.clone(), cell_of(&w, f), f.props.clone()))
                .collect::<Vec<_>>()
        };
        let city = only("counts = { city = 1, town = 0, village = 0, hamlet = 0 }");
        let town = only("counts = { town = 1, village = 0, hamlet = 0 }");
        let both = only("counts = { city = 1, town = 1, village = 0, hamlet = 0 }");
        assert_eq!((city.len(), town.len(), both.len()), (1, 1, 2));
        // Cities and towns score alike, so the first pick of each run is the best site.
        assert_eq!(city[0].1, town[0].1);
        assert_eq!((both[0].0.as_str(), both[0].1), ("city", city[0].1));
        assert_eq!(both[1].0, "town");
        let (a, b) = (both[0].1, both[1].1);
        let d = |x: usize, y: usize| to_f64(x.abs_diff(y)) * 100.0;
        let sep = libm::sqrt(d(a.0, b.0) * d(a.0, b.0) + d(a.1, b.1) * d(a.1, b.1));
        assert!(
            sep >= 600.0,
            "the town keeps 0.6 x its spacing from the city: {sep}"
        );
        let Some(Prop::Int(city_pop)) = both[0].2.get("population") else {
            panic!("population")
        };
        let Some(Prop::Int(town_pop)) = both[1].2.get("population") else {
            panic!("population")
        };
        assert!(city_pop > town_pop);
    }

    #[test]
    fn properties_describe_the_site() {
        let mut w = setup();
        run(&mut w, "counts = { town = 0, village = 0, hamlet = 6 }");
        let found = settlements_of(&w);
        assert_eq!(found.len(), 6);
        for f in &found {
            assert_eq!((f.kind, f.ty.as_str()), ("settlement", "hamlet"));
            let (r, c) = cell_of(&w, f);
            let Some(Prop::Num(e)) = f.props.get("elevation_m") else {
                panic!("elevation")
            };
            let want = 100.0 + 0.25 * to_f64(c);
            assert!((e - want).abs() < 0.051, "{e} {want} at {r},{c}");
            let Some(Prop::Int(pop)) = f.props.get("population") else {
                panic!("population")
            };
            assert!((42..=78).contains(pop), "{pop}");
            let Some(Prop::Text(water)) = f.props.get("water") else {
                panic!("water")
            };
            assert!(["lake", "river", "well"].contains(&water.as_str()));
            assert_eq!(f.props.get("generated_name"), Some(&Prop::Bool(true)));
            assert!(matches!(f.props.get("name"), Some(Prop::Text(_))));
        }
    }

    #[test]
    fn recipe_names_are_used_first_and_only_generated_ones_are_marked() {
        let mut w = setup();
        run(
            &mut w,
            "counts = { town = 3, village = 0, hamlet = 0 }\nnames = [\"Alder\", \"Birch\"]",
        );
        let found = settlements_of(&w);
        let names: Vec<String> = found
            .iter()
            .map(|f| match f.props.get("name") {
                Some(Prop::Text(t)) => t.clone(),
                other => panic!("{other:?}"),
            })
            .collect();
        let from_list = names.iter().filter(|n| *n == "Alder" || *n == "Birch");
        assert_eq!(from_list.count(), 2);
        let generated = found
            .iter()
            .filter(|f| f.props.contains_key("generated_name"))
            .count();
        assert_eq!(generated, 1);
        let unique: std::collections::BTreeSet<_> = names.iter().collect();
        assert_eq!(unique.len(), 3);
    }

    #[test]
    fn existing_features_stay_and_new_ones_follow() {
        let mut w = setup();
        let mut set = FeatureSet::default();
        set.add(Feature {
            kind: "poi",
            ty: "ruin".into(),
            geometry: Geometry::Point([10.0, 10.0]),
            props: std::collections::BTreeMap::new(),
        });
        w.insert("features", Layer::Value(Box::new(set))).unwrap();
        run(&mut w, "counts = { town = 1, village = 0, hamlet = 0 }");
        let all = w.value::<FeatureSet>("features").unwrap();
        assert_eq!(all.len(), 2);
        assert_eq!(all.iter().next().map(|(_, f)| f.ty.as_str()), Some("ruin"));
        assert_eq!(all.count_type("town"), 1);
    }

    #[test]
    fn a_map_without_water_has_no_sites() {
        let mut w = setup();
        w.remove("river");
        w.insert("river", masked(|_, _| false)).unwrap();
        w.remove("lake");
        w.insert("lake", masked(|_, _| false)).unwrap();
        let report = run(&mut w, "");
        assert_eq!(settlements_of(&w), []);
        assert_eq!(report.values.get("settlements"), Some(&0.0));
        let quality = w.f64("site_quality").unwrap();
        assert!(quality.as_slice().iter().all(|q| *q == 0.0));
    }

    #[test]
    fn site_quality_follows_the_formula() {
        let mut w = setup();
        w.insert("river", masked(|_, c| c == 20)).unwrap();
        w.insert("lake", masked(|_, _| false)).unwrap();
        w.insert("big_river", masked(|_, _| false)).unwrap();
        w.insert("slope", flat(0.03)).unwrap();
        let rock = (0..N * N).map(|i| if i % N < 19 { 0 } else { 6 }).collect();
        w.insert("rock", Layer::U8(Grid::from_vec(N, N, rock).unwrap()))
            .unwrap();
        let cold = (0..N * N)
            .map(|i| if i / N == 10 { 2.9 } else { 12.0 })
            .collect();
        w.insert(
            "temperature",
            Layer::F64(Grid::from_vec(N, N, cold).unwrap()),
        )
        .unwrap();
        run(&mut w, "");
        // The noise factor 1 + 0.1 u of the stage's stream, one draw per cell.
        let mut rng = crate::rng::StageRng::for_key(11, "settlements#0/siting");
        let noise: Vec<f64> = (0..N * N).map(|_| 1.0 + 0.1 * rng.uniform()).collect();
        let quality = w.f64("site_quality").unwrap();
        let flat = libm::exp(-0.03 / 0.06);
        let water = |km: f64| libm::exp(-((km - 0.2) / 0.45) * ((km - 0.2) / 0.45));
        // (row, col, distance to the river in km, confluence, soil)
        let cells = [
            (32, 23, 0.3, 1.0, 1.0),
            (32, 17, 0.3, 1.0, 0.0),
            (32, 40, 2.0, 0.0, 1.0),
        ];
        for (r, c, km, confluence, soil) in cells {
            let want =
                flat * water(km) * (0.6 + 0.4 * confluence) * (0.7 + 0.3 * soil) * noise[r * N + c];
            let got = quality.get(r, c);
            assert!((got - want).abs() < 1e-9 * want, "{r},{c}: {got} vs {want}");
        }
        // Too cold: 2.9 degrees is not above 3.
        assert_eq!(quality.get(10, 23), 0.0);
        assert!(quality.get(11, 23) > 0.0);
    }

    #[test]
    fn the_same_inputs_give_the_same_settlements() {
        let (mut a, mut b) = (setup(), setup());
        run(&mut a, "");
        run(&mut b, "");
        assert_eq!(
            a.value::<FeatureSet>("features"),
            b.value::<FeatureSet>("features")
        );
        assert_eq!(a.f64("site_quality"), b.f64("site_quality"));
    }

    #[test]
    fn parameters_have_defaults_and_are_checked() {
        assert!(Settlements.params(&table("")).is_ok());
        let defaults = SettlementsParams::default();
        let counts = Counts {
            city: 0,
            town: 2,
            village: 4,
            hamlet: 4,
        };
        assert_eq!(defaults.counts, counts);
        assert!((defaults.spacing_km.village - 2.5).abs() < 1e-12);
        assert!((defaults.edge_margin - 0.025).abs() < 1e-12);
        assert!(defaults.names.is_none());
        // A counts table replaces the defaults whole, as 0.3.0's dict did.
        let parsed: SettlementsParams = table("counts = { city = 1 }").try_into().unwrap();
        let only_city = Counts {
            city: 1,
            town: 0,
            village: 0,
            hamlet: 0,
        };
        assert_eq!(parsed.counts, only_city);
        let spacing: SettlementsParams = table("spacing_km = { town = 5.0 }").try_into().unwrap();
        assert!((spacing.spacing_km.town - 5.0).abs() < 1e-12);
        assert!((spacing.spacing_km.city - 7.0).abs() < 1e-12);

        for bad in [
            "spacing_km = { city = 0.0 }",
            "spacing_km = { hamlet = -1.0 }",
            "spacing_km = { town = nan }",
            "edge_margin = -0.1",
            "edge_margin = 0.5",
            "edge_margin = nan",
            "names = [\"\"]",
        ] {
            assert!(Settlements.params(&table(bad)).is_err(), "{bad}");
        }
        let err = Settlements.params(&table("edge_margin = 0.7")).unwrap_err();
        assert_eq!(err.parameter.as_deref(), Some("edge_margin"));
    }

    #[test]
    fn unknown_keys_are_refused_also_inside_tables() {
        let err = Settlements.params(&table("spacing = 1.0")).unwrap_err();
        assert!(err.to_string().contains("spacing"), "{err}");
        let err = Settlements
            .params(&table("counts = { cities = 1 }"))
            .unwrap_err();
        assert!(err.to_string().contains("cities"), "{err}");
        let err = Settlements
            .params(&table("spacing_km = { tow = 1.0 }"))
            .unwrap_err();
        assert!(err.to_string().contains("tow"), "{err}");
    }

    #[test]
    fn the_stage_reports_its_counts_under_declared_names() {
        let mut w = setup();
        let report = run(&mut w, "counts = { town = 2, village = 1, hamlet = 0 }");
        let params = Settlements.params(&table("")).unwrap();
        let metrics = Stage::metrics(&Settlements, &w, params.as_ref(), &report);
        let mut names = Stage::metric_names(&Settlements, params.as_ref());
        names.sort_unstable();
        let keys: Vec<&str> = metrics.0.keys().map(String::as_str).collect();
        assert_eq!(names, keys);
        assert_eq!(metrics.0.get("town"), Some(&2.0));
        assert_eq!(metrics.0.get("settlements"), Some(&3.0));
    }

    // Final review round 4, class I: work a recipe count sizes reaches a cancel point (CE-28.4).
    #[test]
    fn a_cancel_stops_the_picking() {
        let mut w = setup();
        let result = run_cancelled(&mut w, &Settlements, "");
        assert!(matches!(result, Err(StageError::Cancelled)), "{result:?}");
    }

    // Final review round 4, class K: the types a stage spells are the table's types of its kind,
    // in both directions.
    #[test]
    fn the_settlement_classes_are_the_tables() {
        let classes: Vec<&str> = CLASSES.iter().map(|c| c.ty).collect();
        let table: Vec<&str> = crate::features::types_of("settlement").collect();
        assert_eq!(classes, table);
    }
}
