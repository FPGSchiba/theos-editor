//! The `space` stage (spec §5.5, §5.7): rooms, walls and doors on the raster.
//!
//! A thin wrapper over [`build_space`]. The ragged noise is one call of P08's `spectral_noise`
//! over the whole frame on `sub_rng("ragged")` (β 2.2, kmin 1); the door positions come from
//! `sub_rng("doors")`. Lengths are metres, whole raster cells.

use serde::{Deserialize, Serialize};

use crate::kernels::noise::spectral_noise;
use crate::rooms::RoomGraph;
use crate::rooms::lattice::Lattice;
use crate::stage::{
    Check, LayerSpec, MapKind, ParamError, StageContext, StageError, StageReport, TypedStage,
};
use crate::stages::interior::space_core::{
    MAX_DOOR, SpaceParams as CoreParams, build_space, out_of_range,
};
use crate::stages::interior::support::{ROOM_GRAPH, SPACE_LAYERS, cells};
use crate::stages::interior::units;
use crate::world::{Layer, MapFrame, ValueType};

/// The spectral exponent of the ragged noise (0.3.0's).
const RAGGED_BETA: f64 = 2.2;

fn one() -> f64 {
    1.0
}

fn two() -> f64 {
    2.0
}

fn half() -> f64 {
    0.5
}

/// The parameters of `space`; lengths in metres, whole raster cells.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct SpaceParams {
    /// The wall thickness, m; at least one cell.
    #[serde(default = "one")]
    pub wall_m: f64,
    /// The door width, m; one or two cells.
    #[serde(default = "one")]
    pub door_m: f64,
    /// The strip width of a `passage` room, m; at least one cell.
    #[serde(default = "two")]
    pub passage_m: f64,
    /// The noise threshold above which a `ragged` room loses an outer cell.
    #[serde(default = "half")]
    pub ragged_t: f64,
}

/// The stage `space`.
pub struct Space;

impl Space {
    /// The core's parameters in cells, with `cells` converting a parameter's metres.
    fn core<E>(
        p: &SpaceParams,
        cells: impl Fn(&str, f64) -> Result<u32, E>,
    ) -> Result<CoreParams, E> {
        Ok(CoreParams {
            wall: cells("wall_m", p.wall_m)?,
            door: cells("door_m", p.door_m)?,
            passage: cells("passage_m", p.passage_m)?,
            ragged_t: p.ragged_t,
        })
    }
}

impl TypedStage for Space {
    type Params = SpaceParams;
    const ID: &'static str = "space";
    const SUMMARY: &'static str =
        "Rooms, walls and doors of an interior on the raster, from its room graph.";
    const KINDS: &'static [MapKind] = &[MapKind::Interior];

    fn reads(&self, _: &SpaceParams) -> Vec<LayerSpec> {
        vec![
            ROOM_GRAPH,
            LayerSpec::value("lattice", Lattice::NAME),
            LayerSpec::mask("envelope"),
        ]
    }

    fn writes(&self, _: &SpaceParams) -> Vec<LayerSpec> {
        SPACE_LAYERS.to_vec()
    }

    fn validate(&self, p: &mut SpaceParams) -> Result<(), ParamError> {
        let check = Check::of(Self::ID);
        check.positive("wall_m", p.wall_m)?;
        check.positive("door_m", p.door_m)?;
        check.positive("passage_m", p.passage_m)?;
        check.finite("ragged_t", p.ragged_t)
    }

    fn check_frame(&self, p: &SpaceParams, frame: &MapFrame) -> Result<(), ParamError> {
        let c = Self::core(p, |name, m| units::cells(Self::ID, name, m, frame))?;
        match out_of_range(&c) {
            Some(("door", _)) => Err(Check::of(Self::ID).refuse(
                "door_m",
                format!("must be from 1 to {MAX_DOOR} raster cells"),
            )),
            Some((name, _)) => Err(Check::of(Self::ID)
                .refuse(&format!("{name}_m"), "must be at least one raster cell")),
            None => Ok(()),
        }
    }

    fn run(&self, ctx: &mut StageContext<'_>, p: &SpaceParams) -> Result<StageReport, StageError> {
        let core = Self::core(p, |name, m| cells(ctx, name, m))?;
        let frame = *ctx.frame();
        let mut ragged = ctx.sub_rng("ragged");
        let noise = spectral_noise(frame.rows, frame.cols, &mut ragged, RAGGED_BETA, 1.0, None);
        let mut doors = ctx.sub_rng("doors");
        let out = {
            let graph = ctx.value::<RoomGraph>("room_graph")?;
            let lattice = ctx.value::<Lattice>("lattice")?;
            let envelope = ctx.mask("envelope")?;
            build_space(graph, lattice, envelope, &core, &noise, &mut doors)
                .map_err(|e| ctx.fail(e.to_string()))?
        };
        ctx.write("rooms", Layer::U32(out.rooms))?;
        ctx.write("walkable", Layer::Mask(out.walkable))?;
        ctx.write("walls", Layer::Mask(out.walls))?;
        ctx.write("corridors", Layer::Mask(out.corridors))?;
        ctx.write("doors", Layer::Value(Box::new(out.doors)))?;
        Ok(StageReport::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stage::Stage;

    fn refusal(text: &str, cell_m: f64) -> Option<String> {
        let frame = MapFrame {
            rows: 40,
            cols: 40,
            cell_m,
        };
        let stage: &dyn Stage = &Space;
        match stage.params(&toml::from_str(text).unwrap()) {
            Err(e) => e.parameter,
            Ok(p) => stage
                .check_frame(p.as_ref(), &frame)
                .err()
                .and_then(|e| e.parameter),
        }
    }

    #[test]
    fn lengths_must_be_whole_cells_in_range() {
        assert_eq!(refusal("", 1.0), None);
        assert_eq!(
            refusal("door_m = 1.0\nwall_m = 0.5\npassage_m = 0.5", 0.5),
            None
        );
        for (text, cell_m, parameter) in [
            ("wall_m = 1.5", 1.0, "wall_m"),
            ("wall_m = 0.0", 1.0, "wall_m"),
            ("door_m = 3.0", 1.0, "door_m"),
            ("door_m = 1.0\nwall_m = 2.0", 2.0, "door_m"),
            ("passage_m = 2.5", 1.0, "passage_m"),
        ] {
            assert_eq!(
                refusal(text, cell_m).as_deref(),
                Some(parameter),
                "{text} at {cell_m}"
            );
        }
    }
}
