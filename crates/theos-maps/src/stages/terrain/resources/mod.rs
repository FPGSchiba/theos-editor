//! The `resources` stage: harvestable deposits from geology, water and ecology, ported from
//! 0.3.0's `features.py::resources`.
//!
//! `fields` computes ten suitability fields (iron ore, copper ore, silver ore, stone, clay,
//! timber, peat and reeds, salt, resonance crystal, herbs), `deposits` places the requested
//! number of each. The fields go to the world value `resource_fields` for the `resources` render.
//! Precipitation comes from `climate.fields` when present, else from the fields computed from
//! `height` and not written, as in `elements`. `omega` is written by a later stage (`detuning`);
//! absent it counts as 0. 0.3.0's lore strings are not in the code; the types are hyphenated.
//!
//! The metric is `deposits`, the number of deposits placed.

mod deposits;
mod fields;
#[cfg(test)]
mod tests;

use std::any::Any;
use std::collections::BTreeMap;

use serde::Deserialize;

use crate::features::FeatureSet;
use crate::grid::Grid;
use crate::kernels::num::to_f64;
use crate::stage::{
    LayerSpec, MapKind, Metrics, ParamError, StageContext, StageError, StageReport, TypedStage,
};
use crate::stages::terrain::climate::{self, Climate};
use crate::stages::terrain::hydrology::WaterFeatures;
use crate::world::{Layer, ValueType, World, WorldValue};

use self::fields::{Inputs, NAMES, compute, terminal_mask};

/// The world value `resource_fields`: the suitability field of each resource type, keyed by the
/// type's hyphenated name.
#[derive(Debug, Clone, PartialEq)]
pub struct ResourceFields {
    /// The fields by type name.
    pub fields: BTreeMap<String, Grid<f64>>,
}

impl WorldValue for ResourceFields {
    fn type_name(&self) -> &'static str {
        Self::NAME
    }
    fn canonical_json(&self) -> Result<serde_json::Value, theos_map_format::FormatError> {
        let fields: BTreeMap<&str, &[f64]> = self
            .fields
            .iter()
            .map(|(name, grid)| (name.as_str(), grid.as_slice()))
            .collect();
        theos_map_format::json::to_checked_value(&fields)
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

impl ValueType for ResourceFields {
    const NAME: &'static str = "resource_fields";
}

/// The parameters of `resources`.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(default)]
pub struct ResourcesParams {
    /// How many deposits of each type, by hyphenated type name; a type left out gets none.
    pub counts: BTreeMap<String, usize>,
    /// The least distance between two deposits of a type, km.
    pub spacing_km: f64,
    /// The margin kept free, a share of the shorter side (spec ruling 46).
    pub edge_margin: f64,
}

impl Default for ResourcesParams {
    fn default() -> Self {
        Self {
            counts: BTreeMap::new(),
            spacing_km: 2.0,
            edge_margin: 0.0125,
        }
    }
}

/// The stage `resources`.
pub struct Resources;

impl TypedStage for Resources {
    type Params = ResourcesParams;
    const ID: &'static str = "resources";
    const SUMMARY: &'static str =
        "Deposits of ore, stone, clay, timber, peat, salt, crystal and herbs on suitable ground.";
    const KINDS: &'static [MapKind] = &[MapKind::Terrain];

    fn reads(&self, _: &ResourcesParams) -> Vec<LayerSpec> {
        vec![
            LayerSpec::f64("height"),
            LayerSpec::f64("slope"),
            LayerSpec::u8("rock"),
            LayerSpec::mask("river"),
            LayerSpec::mask("lake"),
            LayerSpec::u32("lake_id"),
            LayerSpec::value("water_features", WaterFeatures::NAME),
            LayerSpec::f64("vegetation"),
            LayerSpec::f64("temperature"),
            LayerSpec::value("climate", Climate::NAME),
            LayerSpec::f64("element_water"),
            LayerSpec::f64("element_wood"),
            LayerSpec::f64("element_earth"),
            LayerSpec::f64("element_metal"),
            LayerSpec::f64("element_fire"),
            LayerSpec::f64("omega").optional(),
            LayerSpec::value("features", FeatureSet::NAME).optional(),
        ]
    }

    fn writes(&self, _: &ResourcesParams) -> Vec<LayerSpec> {
        vec![
            LayerSpec::value("features", FeatureSet::NAME),
            LayerSpec::value("resource_fields", ResourceFields::NAME),
        ]
    }

    fn metric_names(&self, _: &ResourcesParams) -> Vec<&'static str> {
        vec!["deposits"]
    }

    fn validate(&self, params: &mut ResourcesParams) -> Result<(), ParamError> {
        let at = |name: &str, message: &str| ParamError::at(Self::ID, name, message);
        if let Some(name) = params.counts.keys().find(|k| !NAMES.contains(&k.as_str())) {
            return Err(at(&format!("counts.{name}"), "is not a resource type"));
        }
        let spacing = params.spacing_km;
        if !(spacing.is_finite() && spacing > 0.0) {
            return Err(at("spacing_km", "must be positive and finite"));
        }
        let m = params.edge_margin;
        if !(m.is_finite() && (0.0..0.5).contains(&m)) {
            return Err(at("edge_margin", "must be at least 0 and below 0.5"));
        }
        Ok(())
    }

    fn run(
        &self,
        ctx: &mut StageContext<'_>,
        params: &ResourcesParams,
    ) -> Result<StageReport, StageError> {
        let frame = *ctx.frame();
        let mut rng = ctx.sub_rng("deposits");
        let ruins = ctx
            .value_opt::<FeatureSet>("features")?
            .map_or_else(Vec::new, |set| set.points("poi", Some("ruin")));
        let terminal = terminal_mask(
            ctx.u32("lake_id")?,
            ctx.value::<WaterFeatures>("water_features")?,
        );
        let height = ctx.f64("height")?;
        let climate = ctx.value::<Climate>("climate")?;
        let local;
        let precipitation = if let Some(f) = &climate.fields {
            &f.precipitation
        } else {
            local = climate::fields(height, &frame, &climate.params).precipitation;
            &local
        };
        let inputs = Inputs {
            height,
            slope: ctx.f64("slope")?,
            rock: ctx.u8("rock")?,
            river: ctx.mask("river")?,
            lake: ctx.mask("lake")?,
            terminal: &terminal,
            vegetation: ctx.f64("vegetation")?,
            temperature: ctx.f64("temperature")?,
            precipitation,
            ruins: &ruins,
            omega: ctx.f64_opt("omega")?,
        };
        let computed = compute(&inputs, &frame).map_err(|e| ctx.fail(e.to_string()))?;
        let elements = [
            ctx.f64("element_water")?,
            ctx.f64("element_wood")?,
            ctx.f64("element_earth")?,
            ctx.f64("element_metal")?,
            ctx.f64("element_fire")?,
        ];
        let found = deposits::place(&computed, &elements, &frame, params, &mut rng);
        let mut report = StageReport::default();
        report
            .values
            .insert("deposits".to_owned(), to_f64(found.len()));
        let mut set = if ctx.value_opt::<FeatureSet>("features")?.is_some() {
            *ctx.take_value::<FeatureSet>("features")?
        } else {
            FeatureSet::default()
        };
        for f in found {
            set.add(f);
        }
        let fields = computed
            .into_iter()
            .map(|(name, grid)| (name.to_owned(), grid))
            .collect();
        ctx.write("features", Layer::Value(Box::new(set)))?;
        ctx.write(
            "resource_fields",
            Layer::Value(Box::new(ResourceFields { fields })),
        )?;
        Ok(report)
    }

    fn metrics(&self, _: &World, _: &ResourcesParams, report: &StageReport) -> Metrics {
        Metrics(report.values.clone())
    }
}
