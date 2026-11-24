//! The `elements` stage: the ambient mix of the five elements per cell, ported from 0.3.0's
//! `terrain.py::elements`.
//!
//! * water: `clip(wet + 0.4 moist, 0, 1)`, with `wet = clip(3 gauss(river or lake), 0, 1)` and
//!   `moist = smoothstep(250, 1300, P)`;
//! * wood: the vegetation density;
//! * earth: `(1 − smoothstep(0.05, 0.3, slope))` on soft rock, `× (1 − 0.5 vegetation)`;
//! * metal: `smoothstep(0.15, 0.6, slope)` on hard rock (granite, quartzite, basalt);
//! * fire: `smoothstep(10, 18, T) (1 − moist) + 0.6 [basalt]`, clamped to 1 (0.3.0 let basalt
//!   in a hot, dry place exceed 1).
//!
//! The Gaussian runs over `wet_sigma_m` metres, 0.3.0's 4 cells at 62.5 m. `P` comes from
//! `climate.fields` when present, else from the fields computed from `height` and not written.
//! The metrics `water`, `wood`, `earth`, `metal` and `fire` are each element's share of the
//! mix: its mean over the sum of the five means (all 0 when that sum is 0).

use serde::{Deserialize, Serialize};

use crate::grid::Grid;
use crate::kernels::filters::gaussian;
use crate::kernels::num::{max, min, smoothstep};
use crate::kernels::stats;
use crate::stage::{
    Check, LayerSpec, MapKind, Metrics, ParamError, StageContext, StageError, StageReport,
    TypedStage,
};
use crate::stages::terrain::climate::{self, Climate};
use crate::stages::terrain::rocks::Rock;
use crate::world::{Layer, ValueType, World};

/// The element names, in the order of [`ELEMENT_LAYERS`].
pub(crate) const NAMES: [&str; 5] = ["water", "wood", "earth", "metal", "fire"];

/// The raster of each element (ruling L5), in the order of [`NAMES`].
pub(crate) const ELEMENT_LAYERS: [&str; 5] = [
    "element_water",
    "element_wood",
    "element_earth",
    "element_metal",
    "element_fire",
];

/// The parameters of `elements`.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(default)]
pub struct ElementsParams {
    /// Sigma of the Gaussian that spreads river and lake water to the land around, m.
    pub wet_sigma_m: f64,
}

impl Default for ElementsParams {
    fn default() -> Self {
        Self { wet_sigma_m: 250.0 }
    }
}

/// Whether `code` is a hard rock: granite, quartzite or basalt.
fn is_hard(code: u8) -> bool {
    Rock::from_code(code).is_some_and(Rock::is_hard)
}

/// One cell's inputs.
#[derive(Debug, Clone, Copy)]
pub struct Cell {
    /// Spread river and lake water, `wet` in `[0, 1]`.
    pub wet: f64,
    /// Precipitation, mm/yr.
    pub precipitation: f64,
    /// Vegetation density.
    pub vegetation: f64,
    /// Slope tangent.
    pub slope: f64,
    /// Raster code of the rock.
    pub rock: u8,
    /// Temperature, °C.
    pub temperature: f64,
}

/// The five elements of a cell, each in `[0, 1]`, in the order water, wood, earth, metal, fire.
#[must_use]
pub fn mix(c: &Cell) -> [f64; 5] {
    let moist = smoothstep(250.0, 1300.0, c.precipitation);
    let hard = is_hard(c.rock);
    let basalt = Rock::from_code(c.rock) == Some(Rock::Basalt);
    let earth = if hard {
        0.0
    } else {
        (1.0 - smoothstep(0.05, 0.3, c.slope)) * (1.0 - 0.5 * c.vegetation)
    };
    let metal = if hard {
        smoothstep(0.15, 0.6, c.slope)
    } else {
        0.0
    };
    let fire =
        smoothstep(10.0, 18.0, c.temperature) * (1.0 - moist) + if basalt { 0.6 } else { 0.0 };
    [
        min(max(c.wet + 0.4 * moist, 0.0), 1.0),
        c.vegetation,
        earth,
        metal,
        min(fire, 1.0),
    ]
}

/// The share of each element's mean in the sum of the means; all 0 when the sum is 0.
#[must_use]
pub fn shares(means: [f64; 5]) -> [f64; 5] {
    let total = means.iter().fold(0.0, |sum, m| sum + m);
    if total > 0.0 {
        means.map(|m| m / total)
    } else {
        [0.0; 5]
    }
}

/// The stage `elements`.
pub struct Elements;

impl TypedStage for Elements {
    type Params = ElementsParams;
    const ID: &'static str = "elements";
    const SUMMARY: &'static str = "The ambient mix of water, wood, earth, metal and fire per cell.";
    const KINDS: &'static [MapKind] = &[MapKind::Terrain];

    fn reads(&self, _: &ElementsParams) -> Vec<LayerSpec> {
        vec![
            LayerSpec::f64("height"),
            LayerSpec::mask("river"),
            LayerSpec::mask("lake"),
            LayerSpec::value("climate", Climate::NAME),
            LayerSpec::f64("vegetation"),
            LayerSpec::f64("slope"),
            LayerSpec::u8("rock"),
            LayerSpec::f64("temperature"),
        ]
    }

    fn writes(&self, _: &ElementsParams) -> Vec<LayerSpec> {
        ELEMENT_LAYERS.map(LayerSpec::f64).to_vec()
    }

    fn metric_names(&self, _: &ElementsParams) -> Vec<&'static str> {
        NAMES.to_vec()
    }

    fn validate(&self, params: &mut ElementsParams) -> Result<(), ParamError> {
        Check::of(Self::ID).positive("wet_sigma_m", params.wet_sigma_m)
    }

    fn run(
        &self,
        ctx: &mut StageContext<'_>,
        params: &ElementsParams,
    ) -> Result<StageReport, StageError> {
        let frame = *ctx.frame();
        let climate = ctx.value::<Climate>("climate")?;
        let height = ctx.f64("height")?;
        let local;
        let fields = if let Some(fields) = &climate.fields {
            fields
        } else {
            local = climate::fields(height, &frame, &climate.params);
            &local
        };
        let water = ctx.mask("river")?.map(|r| if r { 1.0 } else { 0.0 });
        let lake = ctx.mask("lake")?;
        let mut source = water;
        for (s, &l) in source.as_mut_slice().iter_mut().zip(lake.as_slice()) {
            if l {
                *s = 1.0;
            }
        }
        let spread = gaussian(&source, params.wet_sigma_m, frame.cell_m);
        let vegetation = ctx.f64("vegetation")?;
        let slope = ctx.f64("slope")?;
        let rock = ctx.u8("rock")?;
        let temperature = ctx.f64("temperature")?;
        let (rows, cols) = (height.rows(), height.cols());
        let mut out: [Grid<f64>; 5] = std::array::from_fn(|_| Grid::filled(rows, cols, 0.0));
        for i in 0..height.len() {
            let cell = Cell {
                wet: min(max(spread.as_slice()[i] * 3.0, 0.0), 1.0),
                precipitation: fields.precipitation.as_slice()[i],
                vegetation: vegetation.as_slice()[i],
                slope: slope.as_slice()[i],
                rock: rock.as_slice()[i],
                temperature: temperature.as_slice()[i],
            };
            for (grid, v) in out.iter_mut().zip(mix(&cell)) {
                grid.as_mut_slice()[i] = v;
            }
        }
        let mut report = StageReport::default();
        // A grid has cells (plan check 1), so the mean exists; 0 stands in for none.
        let means = out
            .each_ref()
            .map(|g| stats::mean(g.as_slice()).unwrap_or(0.0));
        for (name, share) in NAMES.into_iter().zip(shares(means)) {
            report.values.insert(name.to_owned(), share);
        }
        for (name, grid) in ELEMENT_LAYERS.into_iter().zip(out) {
            ctx.write(name, Layer::F64(grid))?;
        }
        Ok(report)
    }

    fn metrics(&self, _: &World, _: &ElementsParams, report: &StageReport) -> Metrics {
        Metrics(report.values.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kernels::num::to_f64;
    use crate::stage::Stage;
    use crate::stages::terrain::climate::ClimateParams;
    use crate::stages::terrain::support::{run_stage, table, world};

    const N: usize = 16;
    const GRANITE: u8 = 0;
    const BASALT: u8 = 2;
    const SHALE: u8 = 5;

    fn near(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-9 * (1.0 + b.abs())
    }

    fn cell(rock: u8) -> Cell {
        Cell {
            wet: 0.0,
            precipitation: 700.0,
            vegetation: 0.5,
            slope: 0.1,
            rock,
            temperature: 12.0,
        }
    }

    #[test]
    fn basalt_feeds_fire() {
        let hot_dry = |rock| Cell {
            precipitation: 100.0,
            temperature: 25.0,
            ..cell(rock)
        };
        let basalt = mix(&hot_dry(BASALT))[4];
        let granite = mix(&hot_dry(GRANITE))[4];
        assert!(near(granite, 1.0) && near(basalt, 1.0), "clamped to 1");
        // On cool ground the basalt bonus shows below the clamp.
        let cool = |rock| Cell {
            precipitation: 100.0,
            temperature: 10.0,
            ..cell(rock)
        };
        assert!(near(mix(&cool(BASALT))[4] - mix(&cool(SHALE))[4], 0.6));
    }

    #[test]
    fn rock_and_slope_split_earth_from_metal() {
        let soft = mix(&Cell {
            slope: 0.0,
            ..cell(SHALE)
        });
        assert!(near(soft[2], 0.75) && soft[3] == 0.0);
        let steep = mix(&Cell {
            slope: 1.0,
            ..cell(GRANITE)
        });
        assert!(near(steep[3], 1.0) && steep[2] == 0.0);
    }

    #[test]
    fn elements_stay_in_the_unit_range() {
        for rock in 0..8 {
            for slope in [0.0, 0.1, 0.4, 2.0] {
                for temperature in [-20.0, 5.0, 14.0, 40.0] {
                    for precipitation in [0.0, 600.0, 3000.0] {
                        let c = Cell {
                            wet: 1.0,
                            precipitation,
                            vegetation: 1.0,
                            slope,
                            rock,
                            temperature,
                        };
                        assert!(mix(&c).iter().all(|e| (0.0..=1.0).contains(e)), "{c:?}");
                    }
                }
            }
        }
    }

    #[test]
    fn shares_sum_to_one_and_vanish_with_an_empty_mix() {
        let s = shares([1.0, 1.0, 2.0, 0.0, 4.0]);
        assert!(near(s.iter().sum::<f64>(), 1.0) && near(s[4], 0.5));
        assert_eq!(shares([0.0; 5]), [0.0; 5]);
    }

    #[test]
    fn parameters_are_checked_and_unknown_keys_refused() {
        let err = Elements.params(&table("wet_sigma_m = 0.0")).unwrap_err();
        assert_eq!(err.parameter.as_deref(), Some("wet_sigma_m"));
        assert!(Elements.params(&table("wet_sigma_m = nan")).is_err());
        let err = Elements.params(&table("wet_sigma = 1.0")).unwrap_err();
        assert!(err.to_string().contains("wet_sigma"), "{err}");
    }

    fn setup(fields: Option<climate::ClimateFields>) -> World {
        let mut w = world(N, N);
        let height = Grid::from_vec(
            N,
            N,
            (0..N * N).map(|i| 100.0 + 10.0 * to_f64(i % N)).collect(),
        )
        .unwrap();
        w.insert("height", Layer::F64(height)).unwrap();
        w.insert(
            "river",
            Layer::Mask(Grid::from_vec(N, N, (0..N * N).map(|i| i % N == 0).collect()).unwrap()),
        )
        .unwrap();
        w.insert("lake", Layer::Mask(Grid::filled(N, N, false)))
            .unwrap();
        w.insert("vegetation", Layer::F64(Grid::filled(N, N, 0.5)))
            .unwrap();
        w.insert("slope", Layer::F64(Grid::filled(N, N, 0.1)))
            .unwrap();
        w.insert("rock", Layer::U8(Grid::filled(N, N, BASALT)))
            .unwrap();
        w.insert("temperature", Layer::F64(Grid::filled(N, N, 12.0)))
            .unwrap();
        let climate = Climate {
            params: ClimateParams::default(),
            fields,
        };
        w.insert("climate", Layer::Value(Box::new(climate)))
            .unwrap();
        w
    }

    #[test]
    fn the_stage_spreads_water_from_rivers_and_reports_shares() {
        let mut w = setup(None);
        let report = run_stage(&mut w, &Elements, "").unwrap();
        assert!(w.value::<Climate>("climate").unwrap().fields.is_none());
        let water = w.f64("element_water").unwrap();
        assert!(
            water.get(8, 0) > water.get(8, 8),
            "water falls off from the river"
        );
        for name in ELEMENT_LAYERS {
            let g = w.f64(name).unwrap();
            assert!(
                g.as_slice().iter().all(|v| (0.0..=1.0).contains(v)),
                "{name}"
            );
        }
        let params = Elements.params(&table("")).unwrap();
        let metrics = Stage::metrics(&Elements, &w, params.as_ref(), &report);
        let mut names = Stage::metric_names(&Elements, params.as_ref());
        names.sort_unstable();
        assert_eq!(
            names,
            metrics.0.keys().map(String::as_str).collect::<Vec<_>>()
        );
        let total: f64 = metrics.0.values().sum();
        assert!(near(total, 1.0), "{total}");

        // The fallback fields equal given fields.
        let height = w.f64("height").unwrap().clone();
        let fields = climate::fields(&height, w.frame(), &ClimateParams::default());
        let mut given = setup(Some(fields));
        run_stage(&mut given, &Elements, "").unwrap();
        assert_eq!(w.f64("element_water"), given.f64("element_water"));
        assert_eq!(w.f64("element_fire"), given.f64("element_fire"));
    }

    #[test]
    fn a_world_with_no_elements_has_zero_shares() {
        let mut w = world(N, N);
        let layers = [
            ("vegetation", 0.0),
            ("slope", 0.0),
            ("temperature", -10.0),
            ("height", 100.0),
        ];
        for (name, v) in layers {
            w.insert(name, Layer::F64(Grid::filled(N, N, v))).unwrap();
        }
        w.insert("river", Layer::Mask(Grid::filled(N, N, false)))
            .unwrap();
        w.insert("lake", Layer::Mask(Grid::filled(N, N, false)))
            .unwrap();
        w.insert("rock", Layer::U8(Grid::filled(N, N, GRANITE)))
            .unwrap();
        let climate = Climate {
            params: ClimateParams::default(),
            fields: Some(climate::ClimateFields {
                precipitation: Grid::filled(N, N, 0.0),
                pet: Grid::filled(N, N, 0.0),
                aet: Grid::filled(N, N, 0.0),
                runoff: Grid::filled(N, N, 0.0),
            }),
        };
        w.insert("climate", Layer::Value(Box::new(climate)))
            .unwrap();
        let report = run_stage(&mut w, &Elements, "").unwrap();
        assert!(report.values.values().all(|v| *v == 0.0) && report.values.len() == 5);
    }

    #[test]
    fn lakes_spread_water_scaled_by_three_and_clipped() {
        let mut w = world(N, N);
        let flat = |v: f64| Layer::F64(Grid::filled(N, N, v));
        for (name, v) in [
            ("vegetation", 0.0),
            ("slope", 0.0),
            ("temperature", 0.0),
            ("height", 100.0),
        ] {
            w.insert(name, flat(v)).unwrap();
        }
        w.insert("river", Layer::Mask(Grid::filled(N, N, false)))
            .unwrap();
        // A lake of 5 x 5 cells and no river.
        let block = |r: usize, c: usize| (6..11).contains(&r) && (6..11).contains(&c);
        let lake: Vec<bool> = (0..N * N).map(|i| block(i / N, i % N)).collect();
        w.insert(
            "lake",
            Layer::Mask(Grid::from_vec(N, N, lake.clone()).unwrap()),
        )
        .unwrap();
        w.insert("rock", Layer::U8(Grid::filled(N, N, SHALE)))
            .unwrap();
        let zero = || Grid::filled(N, N, 0.0);
        let climate = Climate {
            params: ClimateParams::default(),
            fields: Some(climate::ClimateFields {
                precipitation: zero(),
                pet: zero(),
                aet: zero(),
                runoff: zero(),
            }),
        };
        w.insert("climate", Layer::Value(Box::new(climate)))
            .unwrap();
        run_stage(&mut w, &Elements, "").unwrap();

        let source = Grid::from_vec(
            N,
            N,
            lake.iter().map(|l| if *l { 1.0 } else { 0.0 }).collect(),
        )
        .unwrap();
        let g = gaussian(&source, 250.0, 100.0);
        let water = w.f64("element_water").unwrap();
        // Beside the lake the water is positive and clipped; with no rain moist is 0.
        let centre = g.get(8, 8);
        assert!(centre > 1.0 / 3.0 && centre < 1.0, "centre {centre}");
        assert!(near(water.get(8, 8), 1.0));
        // Far from it the clip does not bite, and the value is exactly three times the spread.
        let far = g.get(8, 14);
        assert!(far > 0.0 && 3.0 * far < 1.0, "far {far}");
        assert!(near(water.get(8, 14), 3.0 * far));
    }
}
