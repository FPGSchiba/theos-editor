//! The per-step process `partition_flow`: splits every cell's water over its lower neighbours
//! (or sends it all to the steepest receiver) and orders the cells receivers before donors.
//!
//! The slope-dependent exponent is `FastScape`'s multiple-flow partition (Freeman 1991): the share
//! to neighbour `j` is proportional to `slope_j^p` with `p = 0.5 + 0.6·S`, `S` the steepest
//! slope, so gentle ground spreads the water and steep ground concentrates it. Slopes are
//! `drop / d8 distance` on the `routing_surface`. The stack is Kahn's algorithm over the
//! multiple-flow graph (`stack`).

mod stack;
#[cfg(test)]
mod tests;
mod weights;

use serde::{Deserialize, Serialize};

use crate::grid::Grid;
use crate::stage::{LayerSpec, MapKind, StageContext, StageError, StageReport, TypedStage};
use crate::world::{Layer, ValueType};

use super::flow::FlowWeights;

/// How a cell's water is shared.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Exponent {
    /// Everything goes to the cell's receiver.
    Steepest,
    /// Over all strictly lower D8 neighbours, in proportion to `slope^p`, `p = 0.5 + 0.6·S`.
    SlopeAdaptive,
}

/// The parameters of `partition_flow`.
#[derive(Debug, Deserialize, Serialize)]
pub struct PartitionFlowParams {
    /// How the water is shared; the recipe states it.
    pub exponent: Exponent,
}

/// The process `partition_flow`: writes `weights` and a fresh `stack`.
pub struct PartitionFlow;

impl TypedStage for PartitionFlow {
    type Params = PartitionFlowParams;
    const ID: &'static str = "partition_flow";
    const SUMMARY: &'static str = "Shares each cell's water over its lower neighbours by slope^p (p = 0.5 + 0.6·S, FastScape) or sends it all to the steepest receiver; writes weights and a stack with receivers before donors.";
    const KINDS: &'static [MapKind] = &[MapKind::Terrain];

    fn reads(&self, _: &PartitionFlowParams) -> Vec<LayerSpec> {
        vec![
            LayerSpec::f64("routing_surface"),
            LayerSpec::u32("receivers"),
        ]
    }

    fn writes(&self, _: &PartitionFlowParams) -> Vec<LayerSpec> {
        vec![
            LayerSpec::value("weights", FlowWeights::NAME),
            LayerSpec::u32("stack"),
        ]
    }

    fn run(
        &self,
        ctx: &mut StageContext<'_>,
        params: &PartitionFlowParams,
    ) -> Result<StageReport, StageError> {
        let (surface, receivers) = (ctx.f64("routing_surface")?, ctx.u32("receivers")?);
        let cell_m = ctx.frame().cell_m;
        let built = weights::build(surface, receivers, cell_m, params.exponent);
        let weights = built.map_err(|(cell, to)| {
            ctx.fail(format!(
                "cell {cell} has receiver {to}, which is not a D8 neighbour"
            ))
        })?;
        let order = stack::order(&weights).map_err(|cell| {
            ctx.fail(format!(
                "cell {cell} lies on a cycle of receivers, so no stack orders it"
            ))
        })?;
        let shape = (surface.rows(), surface.cols());
        let stack = Grid::from_vec(shape.0, shape.1, order)
            .map_err(|e| ctx.fail(format!("the stack does not fit the grid: {e}")))?;
        ctx.write("weights", Layer::Value(Box::new(weights)))?;
        ctx.write("stack", Layer::U32(stack))?;
        Ok(StageReport::default())
    }
}
