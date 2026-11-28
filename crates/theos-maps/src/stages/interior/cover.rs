//! The `cover` stage (spec §5.5, §5.8): pillars and rubble that break long sightlines without
//! disconnecting the floor.
//!
//! A thin wrapper over [`place_cover`], which works in metres with the frame's cell size. It
//! reads every layer of `space` (the core takes the rooms, the corridors, the doors and the room
//! centres) and the room graph (round halls get a ring of pillars). The metric is `items`.

use serde::{Deserialize, Serialize};

use crate::grid::Grid;
use crate::rooms::RoomGraph;
use crate::stage::{
    Check, LayerSpec, MapKind, ParamError, StageContext, StageError, StageReport, TypedStage,
};
use crate::stages::interior::cover_core::{CoverParams as CoreParams, place_cover};
use crate::stages::interior::support::{ROOM_GRAPH, SPACE_LAYERS, read_space};
use crate::world::Layer;

/// The parameters of `cover`; lengths in metres.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(default)]
pub struct CoverParams {
    /// The longest straight run of free floor allowed, m.
    pub max_sightline_m: f64,
    /// The most items placed, ring pillars included.
    pub max_items: u32,
    /// The smaller side, m, from which a round hall gets a ring of pillars.
    pub pillar_ring_m: f64,
}

impl Default for CoverParams {
    fn default() -> Self {
        Self {
            max_sightline_m: 24.0,
            max_items: 120,
            pillar_ring_m: 12.0,
        }
    }
}

/// The stage `cover`.
pub struct Cover;

impl TypedStage for Cover {
    type Params = CoverParams;
    const ID: &'static str = "cover";
    const SUMMARY: &'static str =
        "Pillars and rubble that break the long sightlines of an interior.";
    const KINDS: &'static [MapKind] = &[MapKind::Interior];

    fn reads(&self, _: &CoverParams) -> Vec<LayerSpec> {
        let mut reads = vec![ROOM_GRAPH];
        reads.extend(SPACE_LAYERS);
        reads
    }

    fn writes(&self, _: &CoverParams) -> Vec<LayerSpec> {
        vec![LayerSpec::mask("cover")]
    }

    fn metric_names(&self, _: &CoverParams) -> Vec<&'static str> {
        vec!["items"]
    }

    fn validate(&self, p: &mut CoverParams) -> Result<(), ParamError> {
        let check = Check::of(Self::ID);
        check.positive("max_sightline_m", p.max_sightline_m)?;
        check.positive("pillar_ring_m", p.pillar_ring_m)
    }

    fn run(&self, ctx: &mut StageContext<'_>, p: &CoverParams) -> Result<StageReport, StageError> {
        let core = CoreParams {
            max_sightline_m: p.max_sightline_m,
            max_items: p.max_items,
            pillar_ring_m: p.pillar_ring_m,
        };
        let cell_m = ctx.frame().cell_m;
        let (cover, items): (Grid<bool>, u32) = {
            let graph = ctx.value::<RoomGraph>("room_graph")?;
            let space = read_space(ctx, graph.rooms.len())?;
            place_cover(&space, graph, cell_m, &core)
        };
        ctx.write("cover", Layer::Mask(cover))?;
        let mut report = StageReport::default();
        report.put("items", f64::from(items));
        Ok(report)
    }
}
