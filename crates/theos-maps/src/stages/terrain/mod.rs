//! The terrain stages of the outdoor floors, one module per stage.

pub mod climate;
pub mod detuning;
pub mod ecology;
pub mod elements;
pub mod geology;
pub mod grid;
pub mod hydrology;
pub mod pois;
pub mod resources;
pub mod roads;
pub mod rocks;
pub mod settlements;
pub mod sites;
pub mod tectonics;
pub mod travel;

#[cfg(test)]
pub(crate) mod support;

use crate::features::{Feature, FeatureSet};
use crate::grid::Grid;
use crate::kernels::KernelError;
use crate::kernels::distance::distance;
use crate::stage::{StageContext, StageError};
use crate::world::Layer;

/// The distance of every cell to the nearest `river` or `lake` cell, in metres on cells of
/// `cell_m`; `None` on a map without water.
///
/// # Errors
/// [`KernelError`] of the distance transform.
pub(crate) fn water_distance_m(
    river: &Grid<bool>,
    lake: &Grid<bool>,
    cell_m: f64,
) -> Result<Option<Grid<f64>>, KernelError> {
    let mut water = river.clone();
    for (w, &l) in water.as_mut_slice().iter_mut().zip(lake.as_slice()) {
        *w = *w || l;
    }
    if !water.as_slice().iter().any(|&w| w) {
        return Ok(None);
    }
    let (dist, _) = distance(&water, cell_m)?;
    Ok(Some(dist))
}

/// The points of the features of `kind` and type `ty` (`None`: any) in the world value
/// `features`, in metres and insertion order; none when no stage wrote the value yet.
///
/// # Errors
/// [`StageError`] when `features` is not a declared read of the stage or not a feature set.
pub(crate) fn feature_points(
    ctx: &StageContext<'_>,
    kind: &str,
    ty: Option<&str>,
) -> Result<Vec<[f64; 2]>, StageError> {
    Ok(ctx
        .value_opt::<FeatureSet>("features")?
        .map_or_else(Vec::new, |set| set.points(kind, ty)))
}

/// Adds `found`, in order, to the world value `features` (an empty set when no stage wrote it
/// yet) and writes the value back.
///
/// # Errors
/// [`StageError`] when `features` is not a declared read and write of the stage.
pub(crate) fn add_features(
    ctx: &mut StageContext<'_>,
    found: Vec<Feature>,
) -> Result<(), StageError> {
    let mut set = *ctx.take_value_or_default::<FeatureSet>("features")?;
    set.extend(found);
    ctx.write("features", Layer::Value(Box::new(set)))
}
