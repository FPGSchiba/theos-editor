//! The `hydrology` stage (spec §5.9 "Hydrology"): lakes by water balance, channel discharge over
//! the single steepest receiver, river masks, `flow_dir` and the water surface.
//!
//! 1. **Fill** (`discharge::drain`): priority-flood of `height` with ε = 1e-6 (Barnes et al.
//!    2014) from the outlets (`lakes::outlets`): the `fixed` cells, or on a closed map the lowest
//!    cell (ties by index), the map's sink.
//! 2. **Depressions** (`lakes.rs`): 8-connected cells the fill raised by more than 1e-3 m. A
//!    depression's spill level is its highest filled value; its exit is its cell the fill
//!    reached first.
//! 3. **Shallow pits:** a depression shallower than `min_lake_depth_m` (spill level − lowest
//!    height) or smaller than `min_lake_area_m2` is set to the filled surface in `height`, so river
//!    beds never cross dry pits (spec ruling 32).
//! 4. **Inflow:** runoff (mm/yr → m³/yr) is routed over the single steepest receivers of the
//!    filled surface, in reverse flood order, so a depression is settled after every depression
//!    upstream of it (by the flood order of their spill cells, last first). A depression pools
//!    what reaches its cells: `Q_out` from outside them and its own cells' runoff. A terminal
//!    depression passes nothing on.
//! 5. **Lake balance** (Bracht-Flyr et al. 2013, with rain on the lake, rearranged for the area):
//!    `E` is the mean PET and `P` the mean precipitation over the depression (m/yr); `Q_land` is
//!    the runoff from the lake's catchment, land outside the lake. With `E ≤ P`, or when the
//!    whole depression evaporates no more than `Q_out` (`Q_out / (E − P)` reaches its area, and a
//!    full lake leaves no land inside the depression), it fills to spill. Otherwise the lake is
//!    terminal and grows connected from the bottom, one water body: the depression's cells are
//!    taken in the order a priority-flood from the bottom over the depression reaches them (ε,
//!    ties by index), which is spec §5.9's height order for a lake at one level; a nested pit
//!    whose rim lies above the level stays out. The lake is the first `n` of them, the largest
//!    `n` with `n·a·(E − P) ≤ Q_out +` the runoff of the cells after the first `n`, which stay
//!    dry land in the catchment; its level is the fill's value at the `n`-th cell. With `n = 0`
//!    there is no lake, but the depression still keeps its water.
//! 6. **Routing** (`discharge::route`): a second fill from the outlets and the lowest cell of
//!    every terminal depression; the steepest receivers on it give `flow_dir`, and runoff
//!    accumulated over them gives `discharge` in m³/s. A terminal bottom absorbs its inflow; on a
//!    closed map so does the sink. Single flow in channels after Quinn et al. 1991 (spec ruling
//!    27). The second fill starts at a terminal bottom, so it also fills the pits nested in a
//!    terminal depression; those the lake has not reached are dry, even where their floor lies
//!    below the lake level, and are set to the second fill's surface in `height`
//!    (`lakes::fill_dry_pits`), so river beds never cross them either. `Q_out` comes from the
//!    first fill: a few rim cells next to a terminal depression, which drain over its pass on the
//!    first fill, may drain into it on the second (ridge cells with little area; accepted).
//! 7. **Rasters** (`rasters.rs`): `river` and `big_river` where the discharge exceeds `river_q` and
//!    `big_river_q` outside lakes; `lake`, `lake_id` (0 none, else the lake's number in the scan
//!    order of its depression's first cell), `lake_level` (the level on lake cells, else 0) and
//!    `water_surface` (the level on lake cells, else `height`).
//! 8. **Features** (`lines.rs`, `outlines.rs`): the river lines, traced over the same receivers
//!    as the `river` raster and `discharge` and smoothed by two Chaikin passes, with width
//!    `width_a·Q^0.5`, depth `depth_c·Q^0.4` (Leopold and Maddock 1953), the bed height of
//!    `height` and a Strahler order; and the lake outlines as closed rings by marching squares.
//!    Both are the world value `water_features`. `width_a` is 3.0 and `depth_c` is 0.3, for `Q`
//!    in m³/s: mid-range of the hydraulic geometry of mid-latitude rivers, where `a` runs from
//!    about 2 to 5 and `c` from about 0.2 to 0.5.

mod discharge;
mod lakes;
mod lines;
mod outlines;
mod rasters;
#[cfg(test)]
mod tests;

use std::any::Any;

use serde::Deserialize;

use self::discharge::Routing;
use self::lakes::{Basin, Limits};
use self::lines::Geometry;
pub use self::lines::RiverLine;
pub use self::outlines::LakeShape;
use crate::grid::Grid;
use crate::kernels::num::to_f64;
use crate::stage::{
    LayerSpec, MapKind, ParamError, StageContext, StageError, StageReport, TypedStage,
};
use crate::stages::terrain::climate::{self, Climate, ClimateFields};
use crate::world::{Layer, ValueType, WorldValue};

/// A depression smaller than this many cells is filled when the recipe sets no
/// `min_lake_area_m2`.
const MIN_LAKE_CELLS: f64 = 4.0;

/// The world value `water_thresholds`: the discharges above which a cell outside a lake is a
/// river or a big river. `hydrology` writes them from its parameters; later stages read them
/// (a mill needs a multiple of `river_q`).
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct WaterThresholds {
    /// River discharge threshold, m³/s.
    pub river_q: f64,
    /// Big-river discharge threshold, m³/s; not below `river_q`.
    pub big_river_q: f64,
}

impl WorldValue for WaterThresholds {
    fn type_name(&self) -> &'static str {
        Self::NAME
    }
    fn canonical_json(&self) -> serde_json::Value {
        serde_json::json!({"river_q": self.river_q, "big_river_q": self.big_river_q})
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

impl ValueType for WaterThresholds {
    const NAME: &'static str = "water_thresholds";
}

/// The world value `water_features`: the river lines and the lake outlines, in metres.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct WaterFeatures {
    /// The river lines, in the index order of their first cell.
    pub rivers: Vec<RiverLine>,
    /// The lake outlines, in the order of the lake numbers.
    pub lakes: Vec<LakeShape>,
}

impl WorldValue for WaterFeatures {
    fn type_name(&self) -> &'static str {
        Self::NAME
    }
    fn canonical_json(&self) -> serde_json::Value {
        let rivers: Vec<_> = self
            .rivers
            .iter()
            .map(|r| {
                serde_json::json!({
                    "points": r.points,
                    "width_m": r.width_m,
                    "depth_m": r.depth_m,
                    "discharge_m3s": r.discharge_m3s,
                    "order": r.order,
                })
            })
            .collect();
        let lakes: Vec<_> = self
            .lakes
            .iter()
            .map(|l| {
                serde_json::json!({
                    "id": l.id,
                    "rings": l.rings,
                    "surface_m": l.surface_m,
                    "terminal": l.terminal,
                    "outlet": l.outlet,
                    "depth_max_m": l.depth_max_m,
                })
            })
            .collect();
        serde_json::json!({"rivers": rivers, "lakes": lakes})
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

impl ValueType for WaterFeatures {
    const NAME: &'static str = "water_features";
}

/// The parameters of `hydrology`. `river_q` and `big_river_q` have no default (decision 0004):
/// a recipe without them fails naming the parameter.
#[derive(Debug, Deserialize)]
#[serde(default)]
pub struct HydrologyParams {
    /// Discharge above which a cell outside a lake is a river, m³/s; required.
    pub river_q: Option<f64>,
    /// Discharge above which a cell outside a lake is a big river, m³/s; required, not below
    /// `river_q`.
    pub big_river_q: Option<f64>,
    /// A depression shallower than this (spill level − lowest height) is filled, m.
    pub min_lake_depth_m: f64,
    /// A depression smaller than this is filled, m²; four cells when absent.
    pub min_lake_area_m2: Option<f64>,
    /// Width of a river of 1 m³/s, `width = width_a·Q^0.5`, m (Leopold and Maddock 1953: about 2
    /// to 5).
    pub width_a: f64,
    /// Depth of a river of 1 m³/s, `depth = depth_c·Q^0.4`, m (about 0.2 to 0.5).
    pub depth_c: f64,
    /// The thresholds once `validate` has checked them; not a recipe parameter.
    #[serde(skip)]
    pub(crate) thresholds: WaterThresholds,
}

impl Default for HydrologyParams {
    fn default() -> Self {
        Self {
            river_q: None,
            big_river_q: None,
            min_lake_depth_m: 2.0,
            min_lake_area_m2: None,
            width_a: 3.0,
            depth_c: 0.3,
            thresholds: WaterThresholds::default(),
        }
    }
}

impl HydrologyParams {
    /// The smallest depression that keeps its lake, on cells of `cell_area` m².
    fn limits(&self, cell_area: f64) -> Limits {
        Limits {
            min_depth_m: self.min_lake_depth_m,
            min_area_m2: self.min_lake_area_m2.unwrap_or(MIN_LAKE_CELLS * cell_area),
        }
    }
}

/// Everything `hydrology` computes from its inputs: the input of the rasters and, later, of the
/// river lines and lake outlines.
#[derive(Debug)]
pub(crate) struct Water {
    /// `height` with the shallow pits and the dry nested pits of terminal depressions filled.
    pub height: Grid<f64>,
    /// The depressions that keep their water, in the scan order of their first cell.
    pub basins: Vec<Basin>,
    /// How many shallow pits were filled.
    pub filled_pits: usize,
    /// How many cells of dry nested pits in terminal depressions were filled.
    pub filled_nested: usize,
    /// The routing surface, its receivers and the discharge.
    pub routing: Routing,
}

/// Steps 1 to 6 of the stage on `height` with the drained cells `fixed`, the water `fields`
/// (mm/yr) and cells of `cell_m` metres.
pub(crate) fn water(
    height: &Grid<f64>,
    fixed: &Grid<bool>,
    fields: &ClimateFields,
    cell_m: f64,
    limits: &Limits,
) -> Water {
    let cell_area = cell_m * cell_m;
    let runoff = fields.runoff.map(|mm| mm / lakes::MM_PER_M * cell_area);
    let outlets = lakes::outlets(height, fixed);
    let filled = discharge::drain(height, &outlets, cell_m);
    let found = lakes::depressions(height, &filled);
    let mut height = height.clone();
    let (kept, filled_pits) =
        lakes::fill_shallow(&mut height, &filled.surface, found, limits, cell_area);
    let basins = lakes::settle(&filled, kept, &runoff, fields, &height, cell_area);
    let mut seeds = outlets;
    seeds.extend(
        basins
            .iter()
            .filter(|b| b.terminal)
            .map(|b| b.depression.bottom),
    );
    seeds.sort_unstable();
    seeds.dedup();
    let routing = discharge::route(&height, &seeds, &runoff, cell_m);
    let filled_nested = lakes::fill_dry_pits(&mut height, &routing.drainage.surface, &basins);
    Water {
        height,
        basins,
        filled_pits,
        filled_nested,
        routing,
    }
}

/// The counts of `water` and the balance of its terminal depressions (m³/yr, on cells of
/// `cell_area` m²) as report values.
fn report(water: &Water, cell_area: f64) -> StageReport {
    let lakes = water.basins.iter().filter(|b| b.lake.is_some());
    let terminal: Vec<&Basin> = water.basins.iter().filter(|b| b.terminal).collect();
    // Folds from +0.0: an empty f64 sum is -0.0.
    let evaporated = terminal
        .iter()
        .filter_map(|b| {
            let lake = b.lake.as_ref()?;
            let net = b.balance.evaporation - b.balance.precipitation;
            Some(to_f64(lake.cells.len()) * cell_area * net)
        })
        .fold(0.0, |sum, v| sum + v);
    let inflow = terminal.iter().fold(0.0, |sum, b| sum + b.balance.q_land);
    let mut report = StageReport::default();
    let mut put = |name: &str, value: f64| {
        report.values.insert(name.to_owned(), value);
    };
    put("lakes", to_f64(lakes.count()));
    put(
        "terminal_lakes",
        to_f64(terminal.iter().filter(|b| b.lake.is_some()).count()),
    );
    put("filled_pits", to_f64(water.filled_pits));
    put("filled_nested_cells", to_f64(water.filled_nested));
    put("terminal_inflow_m3_yr", inflow);
    put("terminal_evaporation_m3_yr", evaporated);
    report
}

/// The stage `hydrology`.
pub struct Hydrology;

impl TypedStage for Hydrology {
    type Params = HydrologyParams;
    const ID: &'static str = "hydrology";
    const SUMMARY: &'static str = "Lakes by water balance, channel discharge over the single steepest receiver, rivers, flow directions and the water surface.";
    const KINDS: &'static [MapKind] = &[MapKind::Terrain];

    fn reads(&self, _: &HydrologyParams) -> Vec<LayerSpec> {
        vec![
            LayerSpec::f64("height"),
            LayerSpec::value("climate", Climate::NAME),
            LayerSpec::mask("fixed"),
        ]
    }

    fn writes(&self, _: &HydrologyParams) -> Vec<LayerSpec> {
        vec![
            LayerSpec::f64("height"),
            LayerSpec::mask("lake"),
            LayerSpec::u32("lake_id"),
            LayerSpec::f64("lake_level"),
            LayerSpec::mask("river"),
            LayerSpec::mask("big_river"),
            LayerSpec::f64("discharge"),
            LayerSpec::u8("flow_dir"),
            LayerSpec::f64("water_surface"),
            LayerSpec::value("water_thresholds", WaterThresholds::NAME),
            LayerSpec::value("water_features", WaterFeatures::NAME),
        ]
    }

    fn validate(&self, params: &mut HydrologyParams) -> Result<(), ParamError> {
        let at = |name: &str, message: &str| ParamError::at(Self::ID, name, message);
        let threshold = |name: &str, value: Option<f64>| {
            let q = value.ok_or_else(|| at(name, "is required (m³/s); it has no default"))?;
            if q.is_finite() && q > 0.0 {
                Ok(q)
            } else {
                Err(at(name, "must be positive and finite"))
            }
        };
        let river_q = threshold("river_q", params.river_q)?;
        let big_river_q = threshold("big_river_q", params.big_river_q)?;
        if big_river_q < river_q {
            return Err(at("big_river_q", "must not be below river_q"));
        }
        let limits = [
            ("min_lake_depth_m", Some(params.min_lake_depth_m)),
            ("min_lake_area_m2", params.min_lake_area_m2),
            ("width_a", Some(params.width_a)),
            ("depth_c", Some(params.depth_c)),
        ];
        for (name, value) in limits {
            if let Some(v) = value
                && !(v.is_finite() && v >= 0.0)
            {
                return Err(at(name, "must be finite and not negative"));
            }
        }
        params.thresholds = WaterThresholds {
            river_q,
            big_river_q,
        };
        Ok(())
    }

    fn run(
        &self,
        ctx: &mut StageContext<'_>,
        params: &HydrologyParams,
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
        let limits = params.limits(frame.cell_area_m2());
        let water = water(height, ctx.mask("fixed")?, fields, frame.cell_m, &limits);
        let rasters = rasters::rasters(&water, &params.thresholds);
        let report = report(&water, frame.cell_area_m2());
        let geometry = Geometry {
            width_a: params.width_a,
            depth_c: params.depth_c,
        };
        let features = WaterFeatures {
            rivers: lines::lines(&water, &rasters.river, frame.cell_m, &geometry),
            lakes: outlines::outlines(&water, frame.cell_m),
        };
        ctx.write("height", Layer::F64(water.height))?;
        ctx.write("lake", Layer::Mask(rasters.lake))?;
        ctx.write("lake_id", Layer::U32(rasters.lake_id))?;
        ctx.write("lake_level", Layer::F64(rasters.lake_level))?;
        ctx.write("river", Layer::Mask(rasters.river))?;
        ctx.write("big_river", Layer::Mask(rasters.big_river))?;
        ctx.write("discharge", Layer::F64(water.routing.discharge))?;
        ctx.write("flow_dir", Layer::U8(rasters.flow_dir))?;
        ctx.write("water_surface", Layer::F64(rasters.water_surface))?;
        let thresholds = Box::new(params.thresholds);
        ctx.write("water_thresholds", Layer::Value(thresholds))?;
        ctx.write("water_features", Layer::Value(Box::new(features)))?;
        Ok(report)
    }
}
