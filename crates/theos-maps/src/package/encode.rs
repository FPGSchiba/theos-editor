//! The values of one package layer, encoded from the world.

use theos_map_format::{DType, LayerData};

use super::PackageError;
use super::table::{PackageLayer, Source};
use crate::grid::Grid;
use crate::kernels::num::{to_f32, to_u8_unit};
use crate::metrics::precipitation;
use crate::stage::LayerSpec;
use crate::stages::terrain::climate::Climate;
use crate::world::{ValueType, World};

/// The class raster that is all zeros when the recipe has no `roads` stage.
const ROADS: &str = "roads";

/// The water class of a cell (ruling L19): a lake wins over a big river, a big river over a
/// stream.
fn water_class(lake: bool, big_river: bool, river: bool) -> u8 {
    if lake {
        3
    } else if big_river {
        2
    } else {
        u8::from(river)
    }
}

fn mask<'a>(world: &'a World, name: &'static str) -> Result<&'a Grid<bool>, PackageError> {
    world
        .mask(name)
        .ok_or(PackageError::MissingLayer { layer: name })
}

/// The classes of `water`.
fn water(world: &World) -> Result<LayerData, PackageError> {
    let lake = mask(world, "lake")?;
    let big = mask(world, "big_river")?;
    let river = mask(world, "river")?;
    let classes = lake
        .as_slice()
        .iter()
        .zip(big.as_slice())
        .zip(river.as_slice())
        .map(|((&l, &b), &r)| water_class(l, b, r))
        .collect();
    Ok(LayerData::U8(classes))
}

/// The float values of `layer`, from the world or from the climate.
fn floats(world: &World, layer: &PackageLayer) -> Result<Grid<f64>, PackageError> {
    let found = match layer.source {
        Source::Precipitation => precipitation(world),
        _ => world.f64(layer.name).cloned(),
    };
    found.ok_or(PackageError::MissingLayer { layer: layer.name })
}

/// The world layers [`encode`] reads for `layer` (ledger ruling P3): the classes of `water` come
/// from three masks, `precipitation` from the climate (and the height when it holds no fields),
/// `roads` from nothing it cannot do without, every other layer from the layer of its name.
pub(crate) fn needs(layer: &PackageLayer) -> Vec<LayerSpec> {
    match layer.source {
        Source::Water => vec![
            LayerSpec::mask("lake"),
            LayerSpec::mask("big_river"),
            LayerSpec::mask("river"),
        ],
        Source::Precipitation => vec![
            LayerSpec::value("climate", Climate::NAME),
            LayerSpec::f64("height"),
        ],
        Source::Raster => match (layer.dtype, layer.scale) {
            (DType::F32, _) | (_, Some(_)) => vec![LayerSpec::f64(layer.name)],
            (_, None) if layer.name == ROADS => Vec::new(),
            (_, None) => vec![LayerSpec::u8(layer.name)],
        },
    }
}

/// The package values of `layer`: `f32` through `to_f32`, a density through `to_u8_unit`, a
/// class raster as it is.
///
/// # Errors
/// [`PackageError::MissingLayer`] when the world lacks a layer the values come from.
pub(crate) fn encode(world: &World, layer: &PackageLayer) -> Result<LayerData, PackageError> {
    if layer.source == Source::Water {
        return water(world);
    }
    match (layer.dtype, layer.scale) {
        (DType::F32, _) => {
            let grid = floats(world, layer)?;
            Ok(LayerData::F32(
                grid.as_slice().iter().map(|&v| to_f32(v)).collect(),
            ))
        }
        (_, Some(_)) => {
            let grid = floats(world, layer)?;
            Ok(LayerData::U8(
                grid.as_slice().iter().map(|&v| to_u8_unit(v)).collect(),
            ))
        }
        (_, None) => match world.u8(layer.name) {
            Some(grid) => Ok(LayerData::U8(grid.as_slice().to_vec())),
            // A recipe without the `roads` stage has no roads: class 0 everywhere.
            None if layer.name == ROADS => Ok(LayerData::U8(vec![0; world.frame().cells()])),
            None => Err(PackageError::MissingLayer { layer: layer.name }),
        },
    }
}
