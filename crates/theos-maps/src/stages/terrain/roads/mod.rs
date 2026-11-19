//! The `roads` stage (spec §5.9 "Roads"): a road network between the settlements and gates,
//! routed over the terrain in 16 directions, with bridges as single spans and fords on streams.
//!
//! **Steps** (`StepCost`): the 8 neighbours and the 8 knight moves. A step of length `L` and
//! grade `|Δh| / L` costs `L·(1 + (grade / comfort_grade)²·6)` while the grade is within its
//! class's `max_grade`, plus `forest_cost·vegetation·L` on the cell stepped onto, plus
//! `ford_cost` for each stream cell it passes after its first cell. Every cell the step's
//! segment passes through, its ends included, is checked: a step never crosses a lake or
//! big-river cell. A diagonal passes the corner of its two side cells, a knight move the two
//! cells between its ends.
//!
//! **Bridges** (`StepCost::bridges`): from a land cell, a straight line in each of the 16
//! directions across lake or big-river cells to the first land cell on it, up to
//! `max_bridge_span_m` between the bank cells. A bridge costs
//! `span·bridge_cost·(1 + angle_penalty·|cos θ|) + deck_cost·rise`, with θ between the crossing
//! and the flow direction of its first water cell (90° on a lake or where `flow_dir` is 0) and
//! `rise` the deck's height above the lower bank. The deck (`deck_m`) lies `clearance_m` above
//! the higher bank. Bridge steps are exempt from the grade limit.
//!
//! **Network** (own design): the candidate links are the Delaunay edges over the settlements and
//! gates. Each is routed with the class its ends give (0.3.0's rule: the lesser end's rank, a
//! hamlet 1, a village 2, a town or gate 3, a city 4; 3 or more is a main road, 2 a road, 1 a
//! track) and weighted by its cost. Kruskal's tree of those costs is taken, then every other
//! candidate between two settlements of class village or higher, in order of cost, whose cost
//! through the links taken so far exceeds `loop_detour` times its own. The links are laid in
//! order of class (main first), then cost, then their ends' handles; a step on cells that
//! already carry a road costs `merge_discount` times as much, and so does a bridge already
//! built, so trunks merge. Loops are the exception: a loop is routed without the merge discount
//! (ruling 55), so it is a new road and not a copy of the roads it was added to shortcut (0.3.0
//! laid it along them whenever its detour was below `1 / merge_discount`). Tree links and trails
//! keep the discount. Then a trail runs to each ruin from the road cell or place it reaches at
//! the least cost. A place no road reaches, and a ruin no trail reaches, gives one warning each.
//!
//! **Outputs:** the world value `road_network` ([`RoadNetwork`]); the raster `roads`, 0 for no
//! road and the class's code 1 to 4 on every cell a road passes, the higher class where roads
//! overlap (a knight move also marks the cell between its ends that touches both, a bridge every
//! cell under it); and a `bridge` site at the middle of each bridge (`span_m`, `deck_m`) and a
//! `ford` site at the middle stream cell of each stream crossing, each with `road`, the first
//! road that uses it. Cells are flat indices into the frame's grid.
//!
//! **Progress** is reported per routed link: the candidates when costed, the links not taken
//! when the network is chosen, then each link laid and each trail. The searches stop on cancel
//! through the shortest-path kernel.
//!
//! The metrics are the length of each class in km (`main_km`, `road_km`, `track_km`,
//! `trail_km`), the numbers of `bridges` and `fords`, and the number of places no road reaches
//! (`unreached`).

mod bridges;
mod build;
mod cost;
mod crossings;
mod lay;
mod network;
mod params;
mod places;
mod route;
#[cfg(test)]
mod tests;

use std::any::Any;

use serde::Serialize;

pub(crate) use self::build::Layers;
pub(crate) use self::cost::between;
pub(crate) use self::cost::{StepCost, Terrain};
pub(crate) use self::lay::Laid;
pub use self::params::{MaxGrade, RoadsParams, WidthM};
pub(crate) use self::route::Router;
use crate::features::{FeatureHandle, FeatureSet};
use crate::grid::Grid;
use crate::kernels::num::{max, to_f64};
use crate::stage::{
    LayerSpec, MapKind, Metrics, ParamError, StageContext, StageError, StageReport, TypedStage,
};
use crate::world::{Layer, MapFrame, ValueType, World, WorldValue};

/// The class of a road, from the smallest; its code is its value in the `roads` raster.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub enum RoadClass {
    /// A footpath, to ruins (code 1).
    Trail,
    /// A track, to hamlets (code 2).
    Track,
    /// A road between villages and larger places (code 3).
    Road,
    /// A main road between towns, cities and gates (code 4).
    Main,
}

impl RoadClass {
    /// Every class, from the smallest.
    pub const ALL: [Self; 4] = [Self::Trail, Self::Track, Self::Road, Self::Main];

    /// The class's value in the `roads` raster, 1 to 4.
    #[must_use]
    pub const fn code(self) -> u8 {
        match self {
            Self::Trail => 1,
            Self::Track => 2,
            Self::Road => 3,
            Self::Main => 4,
        }
    }

    /// The class's name, the road feature's type (`docs/map-package.md` §5).
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Trail => "trail",
            Self::Track => "track",
            Self::Road => "road",
            Self::Main => "main",
        }
    }
}

/// One road of the network: the cells of its path in order, from its first place to its last.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Road {
    /// The class.
    pub class: RoadClass,
    /// The cells of the path, in order; two cells in a row are one step or one bridge.
    pub cells: Vec<u32>,
    /// The feature the road starts at.
    pub from: FeatureHandle,
    /// The feature the road ends at.
    pub to: FeatureHandle,
    /// The bridges as the pairs of cells they join, in path order.
    pub bridges: Vec<(u32, u32)>,
}

impl Road {
    /// The pairs of cells in a row, as flat indices.
    pub fn steps(&self) -> impl Iterator<Item = (usize, usize)> + '_ {
        self.cells
            .windows(2)
            .map(|pair| (index(pair[0]), index(pair[1])))
    }

    /// Whether the step from `a` to `b` is one of the road's bridges.
    #[must_use]
    pub fn is_bridge(&self, a: usize, b: usize) -> bool {
        self.bridges
            .iter()
            .any(|&(p, q)| index(p) == a && index(q) == b)
    }

    /// The length along the cell centres in metres, bridges included.
    #[must_use]
    pub fn length_m(&self, frame: &MapFrame) -> f64 {
        self.steps()
            .fold(0.0, |sum, (a, b)| sum + distance_m(frame, a, b))
    }

    /// The steepest grade of any step that is not a bridge; 0 for a road of one cell.
    #[must_use]
    pub fn grade_max(&self, height: &Grid<f64>, frame: &MapFrame) -> f64 {
        self.steps()
            .filter(|&(a, b)| !self.is_bridge(a, b))
            .fold(0.0, |steepest, (a, b)| {
                let rise = (height.as_slice()[b] - height.as_slice()[a]).abs();
                max(steepest, rise / distance_m(frame, a, b))
            })
    }
}

/// The world value `road_network`: every road in the order it was laid, and the parameters it
/// was built with, so later stages can route spurs alike and the outputs know the widths.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct RoadNetwork {
    /// The roads, links first in the order laid, then the trails; after `sites`, the spurs it
    /// laid to its sites follow.
    pub roads: Vec<Road>,
    /// The parameters of the `roads` stage that built the network.
    pub params: RoadsParams,
}

/// The feature a path joining the network of `roads` at `cell` meets: a road's end feature, the
/// nearer one along the first road through `cell` (`from` in its first half); `None` when no
/// road passes `cell`.
pub(crate) fn end_near(roads: &[Road], cell: u32) -> Option<FeatureHandle> {
    roads.iter().find_map(|road| {
        let at = road.cells.iter().position(|&c| c == cell)?;
        Some(if 2 * at < road.cells.len() {
            road.from
        } else {
            road.to
        })
    })
}

impl WorldValue for RoadNetwork {
    fn type_name(&self) -> &'static str {
        Self::NAME
    }
    fn canonical_json(&self) -> Result<serde_json::Value, theos_map_format::FormatError> {
        theos_map_format::json::to_checked_value(self)
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

impl ValueType for RoadNetwork {
    const NAME: &'static str = "road_network";
}

/// A cell number of a road as an index.
fn index(cell: u32) -> usize {
    usize::try_from(cell).unwrap_or(usize::MAX)
}

/// The distance between the centres of cells `a` and `b`, m.
pub(crate) fn distance_m(frame: &MapFrame, a: usize, b: usize) -> f64 {
    let (ra, ca) = (a / frame.cols, a % frame.cols);
    let (rb, cb) = (b / frame.cols, b % frame.cols);
    let (dr, dc) = (to_f64(ra.abs_diff(rb)), to_f64(ca.abs_diff(cb)));
    frame.cell_m * libm::sqrt(dr * dr + dc * dc)
}

/// The metric names.
const METRICS: [&str; 7] = [
    "main_km",
    "road_km",
    "track_km",
    "trail_km",
    "bridges",
    "fords",
    "unreached",
];

/// The stage `roads`.
pub struct Roads;

impl TypedStage for Roads {
    type Params = RoadsParams;
    const ID: &'static str = "roads";
    const SUMMARY: &'static str = "Roads between settlements and gates by grade in 16 directions, with bridges, fords and trails to ruins.";
    const KINDS: &'static [MapKind] = &[MapKind::Terrain];

    fn reads(&self, _: &RoadsParams) -> Vec<LayerSpec> {
        vec![
            LayerSpec::f64("height"),
            LayerSpec::f64("vegetation"),
            LayerSpec::mask("river"),
            LayerSpec::mask("big_river"),
            LayerSpec::mask("lake"),
            LayerSpec::u8("flow_dir"),
            LayerSpec::value("features", FeatureSet::NAME).optional(),
        ]
    }

    fn writes(&self, _: &RoadsParams) -> Vec<LayerSpec> {
        vec![
            LayerSpec::value("road_network", RoadNetwork::NAME),
            LayerSpec::u8("roads"),
            LayerSpec::value("features", FeatureSet::NAME),
        ]
    }

    fn metric_names(&self, _: &RoadsParams) -> Vec<&'static str> {
        METRICS.to_vec()
    }

    fn validate(&self, params: &mut RoadsParams) -> Result<(), ParamError> {
        params.check(Self::ID)
    }

    fn run(
        &self,
        ctx: &mut StageContext<'_>,
        params: &RoadsParams,
    ) -> Result<StageReport, StageError> {
        let frame = *ctx.frame();
        let mut set = if ctx.value_opt::<FeatureSet>("features")?.is_some() {
            *ctx.take_value::<FeatureSet>("features")?
        } else {
            FeatureSet::default()
        };
        // The layers are copied so the context stays free for progress events while routing.
        let layers = build::Layers::read(ctx)?;
        let built = build::build(&layers, &set, &frame, params, ctx)?;
        let cost = StepCost::new(layers.terrain(), params, frame.cell_m);
        let found = crossings::sites(&cost, &built.roads, &frame);
        let mut report = StageReport::default();
        let mut put = |name: &str, value: f64| {
            report.values.insert(name.to_owned(), value);
        };
        for class in RoadClass::ALL {
            let metres = built
                .roads
                .iter()
                .filter(|r| r.class == class)
                .fold(0.0, |sum, r| sum + r.length_m(&frame));
            put(&format!("{}_km", class.name()), metres / 1000.0);
        }
        let count = |ty: &str| to_f64(found.iter().filter(|f| f.ty == ty).count());
        put("bridges", count("bridge"));
        put("fords", count("ford"));
        put("unreached", to_f64(built.unreached));
        report.findings = built.warnings;
        for f in found {
            set.add(f);
        }
        let net = RoadNetwork {
            roads: built.roads,
            params: params.clone(),
        };
        ctx.write("road_network", Layer::Value(Box::new(net)))?;
        ctx.write("roads", Layer::U8(built.raster))?;
        ctx.write("features", Layer::Value(Box::new(set)))?;
        Ok(report)
    }

    fn metrics(&self, _: &World, _: &RoadsParams, report: &StageReport) -> Metrics {
        Metrics(report.values.clone())
    }
}
