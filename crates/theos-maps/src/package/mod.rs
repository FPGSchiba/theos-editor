//! Building the map package of a run (spec §5.10, `docs/map-package.md` §3-§4).

mod encode;
mod table;
#[cfg(test)]
mod tests;

use std::collections::BTreeMap;

use theos_map_format::{
    FORMAT, FORMAT_VERSION, Frame, LayerEntry, LayerKey, Manifest, Package, Parent,
};

use self::encode::encode;
pub(crate) use self::table::{OMEGA_LAYER, PackageLayer, TERRAIN_LAYERS};
use crate::api::{GENERATOR, VERSION};
use crate::kind;
use crate::recipe::Plan;
use crate::stage::{LayerSpec, MapKind};
use crate::world::World;

/// Why a package could not be built.
#[derive(Debug, thiserror::Error)]
pub enum PackageError {
    /// A layer the package needs is absent from the world or has another type.
    #[error("the world has no usable layer `{layer}` for the package")]
    MissingLayer {
        /// The layer name.
        layer: &'static str,
    },
    /// The grid does not fit the manifest's 32-bit rows and columns.
    #[error("the grid of {rows} x {cols} cells is too large for a package")]
    GridTooLarge {
        /// Rows of the frame.
        rows: usize,
        /// Columns of the frame.
        cols: usize,
    },
    /// The recipe cannot be written as JSON: it holds a NaN or an infinity (spec §5.13), or a
    /// value JSON has no form for.
    #[error("the recipe cannot be written as JSON: {0}")]
    Recipe(#[from] theos_map_format::FormatError),
}

/// What a finished run adds to the layers of its world: the manifest's recipe, seed, inputs,
/// results and summary, and the features.
pub(crate) struct RunRecord<'a> {
    /// The recipe as it was loaded.
    pub raw_recipe: &'a toml::Table,
    /// The run's seed.
    pub seed: u64,
    /// The run's inputs.
    pub inputs: &'a serde_json::Map<String, serde_json::Value>,
    /// The run's results.
    pub results: serde_json::Map<String, serde_json::Value>,
    /// The run summary.
    pub summary: serde_json::Value,
    /// The features.
    pub features: geojson::FeatureCollection,
}

/// The manifest entry of `layer`; the writer fills in file, shape, hash and range.
fn entry(layer: &PackageLayer, shape: [u32; 2]) -> LayerEntry {
    LayerEntry {
        name: layer.name.to_owned(),
        file: String::new(),
        dtype: layer.dtype,
        shape,
        sha256: String::new(),
        description: layer.description.to_owned(),
        units: layer.units.map(str::to_owned),
        scale: layer.scale,
        offset: None,
        legend: layer.legend.map(|pairs| {
            pairs
                .iter()
                .map(|&(value, meaning)| (value.to_owned(), meaning.to_owned()))
                .collect::<BTreeMap<_, _>>()
        }),
        min: None,
        max: None,
        level: None,
    }
}

/// The world layers the package reads whatever the recipe (ledger ruling P3): those of every
/// standard layer; `roads` is all zeros without the `roads` stage (Task 42) and `omega` is
/// written only when the world holds it, so neither is needed.
pub(crate) fn needs(kind: MapKind) -> Vec<LayerSpec> {
    kind::table(kind)
        .layers
        .iter()
        .flat_map(encode::needs)
        .collect()
}

/// The package of a finished run: the standard layers in order (`omega` last, when the
/// world holds it), the manifest with the recipe as it ran, and the features.
///
/// `generator` is `theos-maps@` plus the generator's version, and `requires` is always empty
/// (spec ruling 47).
///
/// # Errors
/// [`PackageError`] when the world lacks a required layer, the grid is too large for the
/// manifest, or the recipe cannot be written as JSON (a NaN or an infinity in it).
pub(crate) fn build(
    world: &World,
    plan: &Plan,
    run: RunRecord<'_>,
) -> Result<Package, PackageError> {
    let frame = world.frame();
    let too_large = || PackageError::GridTooLarge {
        rows: frame.rows,
        cols: frame.cols,
    };
    let shape = [
        u32::try_from(frame.rows).map_err(|_| too_large())?,
        u32::try_from(frame.cols).map_err(|_| too_large())?,
    ];
    let table = kind::table(plan.kind);
    let optional = table
        .optional_layers
        .iter()
        .filter(|layer| world.f64(layer.name).is_some());
    let mut entries = Vec::new();
    let mut layers = BTreeMap::new();
    for layer in table.layers.iter().chain(optional) {
        layers.insert(
            LayerKey {
                level: None,
                name: layer.name.to_owned(),
            },
            encode(world, layer)?,
        );
        entries.push(entry(layer, shape));
    }
    let manifest = Manifest {
        format: FORMAT.to_owned(),
        format_version: FORMAT_VERSION.to_owned(),
        generator: format!("{GENERATOR}@{VERSION}"),
        requires: Vec::new(),
        kind: plan.kind.to_string(),
        recipe_id: plan.recipe_id.clone(),
        recipe: theos_map_format::json::to_checked_value(run.raw_recipe)?,
        seed: run.seed,
        inputs: run.inputs.clone(),
        frame: Frame {
            size_m: frame.size_m(),
            cell_m: frame.cell_m,
            grid: shape,
            origin_m: plan.origin_m,
            parent: plan.parent.as_ref().map(|parent| Parent {
                package_sha256: parent.package_sha256.clone(),
                feature: parent.feature.clone(),
            }),
            levels: None,
        },
        layers: entries,
        features: String::new(),
        results: run.results,
        summary: run.summary,
    };
    Ok(Package {
        manifest,
        features: run.features,
        layers,
    })
}
