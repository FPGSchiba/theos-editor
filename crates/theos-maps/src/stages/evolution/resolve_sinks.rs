//! The per-step process `resolve_sinks`: routes every closed basin out through its lowest pass,
//! after Cordonnier, Bovy & Braun 2019 (the minimum spanning tree of basins) and Barnes, Lehman &
//! Mulla 2014 (the ε lift); spec §5.9 and rulings 4 and 26.
//!
//! 1. **Outlets** are the `fixed` cells; with none, the lowest cell (ties by index) is the map's
//!    sink (`basins`).
//! 2. **Basins:** each cell belongs to the outlet or pit its receiver chain ends in; every outlet
//!    belongs to the one root basin (`basins`).
//! 3. **Basin graph:** over every D8 pair of cells in two basins, the pass height is
//!    `max(h_a, h_b)`; the lowest pass per basin pair is kept, ties by the smaller
//!    `(min cell, max cell)` (`tree`).
//! 4. **Tree:** Kruskal over the basins, oriented to the root: each closed basin drains over its
//!    pass from its cell `a` to the cell `b` of its parent, at the pass height `L` (`tree`).
//! 5. **Depressions:** from the root outwards, each closed basin is lifted on the
//!    `routing_surface` by a priority flood with ε = 1e-6 from `a`; the lifted cells form a
//!    depression with `SpillPoint { cell: a, to: b, level: L }`. When the flood starts above `L`
//!    (the lake of `b` stands higher), the basin joins the lake of `b` instead and adds no spill
//!    point. `height` is unchanged (`lift`).
//! 6. **Receivers again:** steepest descent on the routing surface, and each `a` receives `b`.
//!
//! Two details keep every route open. A basin whose `b` lies in a lifted depression starts one ε
//! above `b`'s routing level, not below it (a pit nested in a lake drains at the lake's level). And
//! a cell just above the spill level next to the lifted cells is lifted too, so it keeps a lower
//! neighbour. Then every receiver lies lower on the routing surface than its cell, except over
//! a pass, where `b` is not higher than `a`; so every chain ends at an outlet without a cycle.
//!
//! A basin whose pass lies below the lake it spills into is part of that lake: its lifted cells
//! take the lake's depression and add no spill point. A depression is thus one lake, which
//! `fluvial` fills as one up to the level it spills at (spec §5.9 "Depression filling"). As a
//! depression of its own it would take sediment only up to its pass, and the lake's room
//! between that pass and the lake's level would never fill (found in the build, Task 43e).

mod basins;
mod depressions;
mod lift;
#[cfg(test)]
mod tests;
mod tree;

use crate::kernels::num::to_u32;
use crate::kernels::routing::steepest_receivers;
use serde::Deserialize;

pub(crate) use self::depressions::Depressions;
// The tests of the processes after this one build spill points.
#[cfg(test)]
pub(crate) use self::depressions::SpillPoint;
use crate::stage::{LayerSpec, MapKind, StageContext, StageError, StageReport, TypedStage};
use crate::world::{Layer, ValueType};

/// The parameters of `resolve_sinks`: none.
#[derive(Debug, Default, Deserialize)]
pub struct ResolveSinksParams {}

/// The process `resolve_sinks`: connects every closed basin through its lowest pass and writes
/// the routing surface, the corrected receivers and the depressions.
pub struct ResolveSinks;

impl TypedStage for ResolveSinks {
    type Params = ResolveSinksParams;
    const ID: &'static str = "resolve_sinks";
    const SUMMARY: &'static str = "Connects each closed basin through its lowest pass by the minimum spanning tree of basins; the routing surface lifts depressions to their spill level + ε along the priority-flood order, height unchanged.";
    const KINDS: &'static [MapKind] = &[MapKind::Terrain];

    fn reads(&self, _: &ResolveSinksParams) -> Vec<LayerSpec> {
        vec![
            LayerSpec::f64("height"),
            LayerSpec::u32("receivers"),
            LayerSpec::mask("fixed"),
        ]
    }

    fn writes(&self, _: &ResolveSinksParams) -> Vec<LayerSpec> {
        vec![
            LayerSpec::u32("receivers"),
            LayerSpec::f64("routing_surface"),
            LayerSpec::value("depressions", Depressions::NAME),
        ]
    }

    fn run(
        &self,
        ctx: &mut StageContext<'_>,
        _: &ResolveSinksParams,
    ) -> Result<StageReport, StageError> {
        let (height, receivers, fixed) = (
            ctx.f64("height")?,
            ctx.u32("receivers")?,
            ctx.mask("fixed")?,
        );
        if let Some((cell, to)) = basins::bad_receiver(height, receivers) {
            return Err(ctx.fail(format!(
                "cell {cell} has receiver {to}, which is neither the cell itself nor a lower D8 neighbour"
            )));
        }
        let basins = basins::basins(height, receivers, fixed);
        let links = tree::links(height, &basins);
        let (surface, depressions) = lift::lift(height, &basins, &links);
        let mut receivers = steepest_receivers(&surface, fixed, ctx.frame().cell_m);
        for link in &links {
            receivers.as_mut_slice()[link.a] = to_u32(link.b);
        }
        ctx.write("receivers", Layer::U32(receivers))?;
        ctx.write("routing_surface", Layer::F64(surface))?;
        ctx.write("depressions", Layer::Value(Box::new(depressions)))?;
        Ok(StageReport::default())
    }
}
