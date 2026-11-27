//! What the interior stages share: lengths in cells at run time, and the rasters of `space` read
//! back from the world.

use crate::rooms::RoomGraph;
use crate::stage::{LayerSpec, StageContext, StageError};
use crate::stages::interior::space_core::{Doors, SpaceOut, room_centres};
use crate::stages::interior::units;
use crate::world::ValueType;

/// The layers `space` writes, as the later stages read them back.
pub(super) const SPACE_LAYERS: [LayerSpec; 5] = [
    LayerSpec::u32("rooms"),
    LayerSpec::mask("walkable"),
    LayerSpec::mask("walls"),
    LayerSpec::mask("corridors"),
    LayerSpec::value("doors", Doors::NAME),
];

/// The room graph's declaration, as `mission` writes it and the later stages read it.
pub(super) const ROOM_GRAPH: LayerSpec = LayerSpec::value("room_graph", RoomGraph::NAME);

/// `metres` of `parameter` in raster cells at run time. The plan has checked it with the same
/// [`units::cells`], so an error here means the frame changed after planning.
///
/// # Errors
/// [`StageError::Failed`] naming the stage and the parameter.
pub(super) fn cells(
    ctx: &StageContext<'_>,
    parameter: &str,
    metres: f64,
) -> Result<u32, StageError> {
    units::cells(ctx.stage(), parameter, metres, ctx.frame()).map_err(|e| ctx.fail(e.to_string()))
}

/// The rasters and doors `space` wrote, with the room centres computed from `rooms` for the
/// graph's `n` rooms (the same centres `space` used, [`room_centres`]).
///
/// # Errors
/// [`StageError`] when a layer of [`SPACE_LAYERS`] is not declared or missing.
pub(super) fn read_space(ctx: &StageContext<'_>, n: usize) -> Result<SpaceOut, StageError> {
    let rooms = ctx.u32("rooms")?.clone();
    let centres = room_centres(&rooms, n);
    Ok(SpaceOut {
        walkable: ctx.mask("walkable")?.clone(),
        walls: ctx.mask("walls")?.clone(),
        corridors: ctx.mask("corridors")?.clone(),
        doors: ctx.value::<Doors>("doors")?.clone(),
        rooms,
        centres,
    })
}
