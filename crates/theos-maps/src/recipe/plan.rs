//! Validation of a recipe before any stage runs (spec §5.5). Nothing runs on an error.

use std::collections::{BTreeMap, BTreeSet};

use super::{LoadedRecipe, RecipeError};
use crate::constraints::is_known_path;
use crate::kernels::num;
use crate::render::{self, Role};
use crate::stage::{LayerSpec, MapKind, ParamError, Registry, StageParams};
use crate::world::{LayerKind, MapFrame};

/// How far two cell edges may differ, relative, and still count as square.
const SQUARE_TOLERANCE: f64 = 1e-9;

/// The smallest cell edge of a map, m: a centimetre, below any floor plan's need.
pub const MIN_CELL_M: f64 = 0.01;

/// The largest cell edge of a map, m: 100 km, beyond any world map's need.
pub const MAX_CELL_M: f64 = 100_000.0;

/// A validated recipe: the frame and the stages with their parsed parameters.
#[derive(Debug)]
pub struct Plan {
    /// The recipe id.
    pub recipe_id: String,
    /// The kind of map.
    pub kind: MapKind,
    /// The frame the recipe's grid and size give.
    pub frame: MapFrame,
    /// The stages in order.
    pub stages: Vec<PlannedStage>,
    /// Constraint ranges by summary path; check 5 has made sure that every path is known and
    /// every pair of bounds finite and ordered.
    pub constraints: BTreeMap<String, [f64; 2]>,
    /// The extra renders asked for.
    pub renders: Vec<String>,
    /// The options per render.
    pub render_options: BTreeMap<String, toml::Table>,
}

/// One stage of a [`Plan`].
#[derive(Debug)]
pub struct PlannedStage {
    /// The stage id.
    pub id: &'static str,
    /// `<id>#<occurrence>`, the occurrence counted per id from 0; it names the stage's random
    /// stream.
    pub key: String,
    /// The parsed parameters.
    pub params: Box<dyn StageParams>,
    /// The layers the stage reads with these parameters.
    pub reads: Vec<LayerSpec>,
    /// The layers the stage writes with these parameters.
    pub writes: Vec<LayerSpec>,
    /// The names of the metrics the stage reports.
    pub metric_names: Vec<&'static str>,
}

/// Where a refusal lies.
#[derive(Default)]
struct Spot<'a> {
    stage: Option<&'a str>,
    layer: Option<&'a str>,
    parameter: Option<&'a str>,
}

fn invalid(recipe: &LoadedRecipe, spot: &Spot<'_>, message: impl Into<String>) -> RecipeError {
    RecipeError::Invalid {
        recipe: recipe.recipe.id.clone(),
        stage: spot.stage.map(str::to_owned),
        layer: spot.layer.map(str::to_owned),
        parameter: spot.parameter.map(str::to_owned),
        message: message.into(),
    }
}

fn at_parameter(parameter: &str) -> Spot<'_> {
    Spot {
        parameter: Some(parameter),
        ..Spot::default()
    }
}

/// Validates `recipe` against `registry` and the request's `inputs`, in the order of spec §5.5:
/// id, frame, renders, inputs, then each stage (known, kind, parameters, order, reads), then the
/// constraints.
///
/// # Errors
/// [`RecipeError::Invalid`] naming the recipe and, where known, the stage, layer and parameter.
pub fn plan(
    recipe: &LoadedRecipe,
    registry: &Registry,
    inputs: &serde_json::Map<String, serde_json::Value>,
) -> Result<Plan, RecipeError> {
    check_id(recipe)?;
    let frame = check_frame(recipe)?;
    check_renders(recipe)?;
    check_inputs(recipe, inputs)?;
    let stages = plan_stages(recipe, registry)?;
    check_constraints(recipe, &stages)?;
    let r = &recipe.recipe;
    Ok(Plan {
        recipe_id: r.id.clone(),
        kind: r.kind,
        frame,
        stages,
        constraints: r.constraints.clone(),
        renders: r.renders.clone(),
        render_options: r.render.clone(),
    })
}

/// Check 0: the id names output folders and manifest fields, `^[a-z0-9][a-z0-9-]*$`.
fn check_id(recipe: &LoadedRecipe) -> Result<(), RecipeError> {
    let id = &recipe.recipe.id;
    let lower_or_digit = |c: char| c.is_ascii_lowercase() || c.is_ascii_digit();
    let mut chars = id.chars();
    let valid =
        chars.next().is_some_and(lower_or_digit) && chars.all(|c| lower_or_digit(c) || c == '-');
    if valid {
        return Ok(());
    }
    Err(invalid(
        recipe,
        &at_parameter("id"),
        format!("recipe id `{id}` must match ^[a-z0-9][a-z0-9-]*$"),
    ))
}

/// Check 5: constraint bounds are finite and ordered, and the paths exist.
fn check_constraints(recipe: &LoadedRecipe, stages: &[PlannedStage]) -> Result<(), RecipeError> {
    for (path, [min, max]) in &recipe.recipe.constraints {
        if !(min.is_finite() && max.is_finite() && min <= max) {
            return Err(invalid(
                recipe,
                &at_parameter(path),
                format!(
                    "constraint `{path}` needs finite bounds with min <= max, got [{min}, {max}]"
                ),
            ));
        }
        if !is_known_path(path, recipe.recipe.kind, stages) {
            return Err(invalid(
                recipe,
                &at_parameter(path),
                format!("constraint path `{path}` is not a known summary path of this recipe"),
            ));
        }
    }
    Ok(())
}

/// Check 1: the keys `grid` and `size_km` by kind (terrain and interior need both, a settlement
/// map has neither: its frame comes from the parent), then the grid (at least 2 x 2, a cell count that fits a `usize`), the size, square cells
/// and a cell edge from [`MIN_CELL_M`] to [`MAX_CELL_M`].
fn check_frame(recipe: &LoadedRecipe) -> Result<MapFrame, RecipeError> {
    let r = &recipe.recipe;
    let (grid, size_km) = match r.kind {
        MapKind::Settlement => {
            for (key, present) in [("grid", r.grid.is_some()), ("size_km", r.size_km.is_some())] {
                if present {
                    return Err(invalid(
                        recipe,
                        &at_parameter(key),
                        format!(
                            "a settlement recipe has no `{key}`; its frame comes from the parent"
                        ),
                    ));
                }
            }
            return Err(invalid(
                recipe,
                &Spot::default(),
                "the frame of a settlement map is not available yet",
            ));
        }
        MapKind::Terrain | MapKind::Interior => {
            let need = |key: &str| {
                invalid(
                    recipe,
                    &at_parameter(key),
                    format!("a {} recipe needs `{key}`", r.kind),
                )
            };
            (
                r.grid.ok_or_else(|| need("grid"))?,
                r.size_km.ok_or_else(|| need("size_km"))?,
            )
        }
    };
    let [rows, cols] = grid;
    if rows < 2 || cols < 2 {
        return Err(invalid(
            recipe,
            &at_parameter("grid"),
            format!("`grid` must be at least 2 x 2, got {rows} x {cols}"),
        ));
    }
    // Every raster and `MapFrame::cells` multiply the two; the memory budget sees only the product.
    if rows.checked_mul(cols).is_none() {
        return Err(invalid(
            recipe,
            &at_parameter("grid"),
            format!("`grid` of {rows} x {cols} overflows the cell count"),
        ));
    }
    let [width, height] = size_km;
    // In metres too: a size beyond f64 there would make the cell edge an infinity.
    let usable = |km: f64| km > 0.0 && (km * 1000.0).is_finite();
    if !(usable(width) && usable(height)) {
        return Err(invalid(
            recipe,
            &at_parameter("size_km"),
            "`size_km` must be positive and finite, also in metres",
        ));
    }
    let across = width / num::to_f64(cols);
    let down = height / num::to_f64(rows);
    if (across - down).abs() > SQUARE_TOLERANCE * across {
        return Err(invalid(
            recipe,
            &at_parameter("size_km"),
            format!(
                "cells must be square: {width} km over {cols} columns is {} m, \
                 {height} km over {rows} rows is {} m",
                across * 1000.0,
                down * 1000.0
            ),
        ));
    }
    // Final review round 3, class B: every metre of a stage becomes cells, areas and slopes
    // through the cell edge, so its range keeps them finite and its conversions bounded. It also
    // refuses a size whose cells underflow to 0, which the square check above cannot see.
    let cell_m = width * 1000.0 / num::to_f64(cols);
    if !(MIN_CELL_M..=MAX_CELL_M).contains(&cell_m) {
        return Err(invalid(
            recipe,
            &at_parameter("size_km"),
            format!(
                "`size_km` over `grid` gives cells of {cell_m} m; the edge must be from \
                 {MIN_CELL_M} m to {MAX_CELL_M} m"
            ),
        ));
    }
    Ok(MapFrame { rows, cols, cell_m })
}

/// Check 2: the extra renders and the render options.
fn check_renders(recipe: &LoadedRecipe) -> Result<(), RecipeError> {
    let r = &recipe.recipe;
    let extras: Vec<&str> = render::names(r.kind, Role::Extra).collect();
    if let Some(name) = r.renders.iter().find(|n| !extras.contains(&n.as_str())) {
        return Err(invalid(
            recipe,
            &at_parameter("renders"),
            format!(
                "unknown extra render `{name}` (known: {})",
                extras.join(", ")
            ),
        ));
    }
    for id in r.render.keys() {
        if render::needs(r.kind, id).is_none() {
            let key = format!("render.{id}");
            return Err(invalid(
                recipe,
                &at_parameter(&key),
                format!("`[render.{id}]` names no render"),
            ));
        }
    }
    Ok(())
}

/// Check 3: the request's inputs against the recipe's declarations.
fn check_inputs(
    recipe: &LoadedRecipe,
    request: &serde_json::Map<String, serde_json::Value>,
) -> Result<(), RecipeError> {
    recipe
        .recipe
        .inputs()
        .check(request)
        .map_err(|e| invalid(recipe, &at_parameter(&e.input), e.message))
}

/// Check 4: the stages, in order, with the layers available before each.
fn plan_stages(
    recipe: &LoadedRecipe,
    registry: &Registry,
) -> Result<Vec<PlannedStage>, RecipeError> {
    let mut available: BTreeMap<&'static str, LayerKind> = BTreeMap::new();
    let mut seen: BTreeMap<&'static str, u32> = BTreeMap::new();
    let mut stages = Vec::new();
    for table in &recipe.recipe.stages {
        let Some(id) = table.get("id").and_then(toml::Value::as_str) else {
            return Err(invalid(recipe, &Spot::default(), "a stage has no `id`"));
        };
        let on_stage = Spot {
            stage: Some(id),
            ..Spot::default()
        };
        let Some(stage) = registry.get(id) else {
            return Err(invalid(recipe, &on_stage, format!("unknown stage `{id}`")));
        };
        if !stage.kinds().contains(&recipe.recipe.kind) {
            return Err(invalid(
                recipe,
                &on_stage,
                format!("stage `{id}` does not apply to {} maps", recipe.recipe.kind),
            ));
        }
        let param_error = |e: ParamError| {
            invalid(
                recipe,
                &Spot {
                    stage: Some(id),
                    layer: None,
                    parameter: e.parameter.as_deref(),
                },
                e.message,
            )
        };
        let params = stage.params(table).map_err(param_error)?;
        let names: BTreeSet<&str> = available.keys().copied().collect();
        stage
            .check_order(params.as_ref(), &names)
            .map_err(param_error)?;
        let reads = stage.reads(params.as_ref());
        for read in &reads {
            let on_layer = Spot {
                stage: Some(id),
                layer: Some(read.name),
                parameter: None,
            };
            match available.get(read.name) {
                Some(kind) if *kind != read.kind => {
                    return Err(invalid(
                        recipe,
                        &on_layer,
                        format!(
                            "stage `{id}` reads `{}` as {}, but an earlier stage writes it as {kind}",
                            read.name, read.kind
                        ),
                    ));
                }
                Some(_) => {}
                None if read.optional => {}
                None => {
                    return Err(invalid(
                        recipe,
                        &on_layer,
                        format!(
                            "stage `{id}` reads `{}`, which no earlier stage writes",
                            read.name
                        ),
                    ));
                }
            }
        }
        let writes = stage.writes(params.as_ref());
        available.extend(
            writes
                .iter()
                .filter(|w| !w.optional)
                .map(|w| (w.name, w.kind)),
        );
        let stage_id = stage.id();
        let occurrence = seen.entry(stage_id).or_insert(0);
        let key = format!("{stage_id}#{occurrence}");
        *occurrence += 1;
        stages.push(PlannedStage {
            id: stage_id,
            key,
            metric_names: stage.metric_names(params.as_ref()),
            params,
            reads,
            writes,
        });
    }
    Ok(stages)
}

#[cfg(test)]
mod tests;
