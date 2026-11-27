//! Validation of a recipe before any stage runs (spec §5.5). Nothing runs on an error.

use std::collections::{BTreeMap, BTreeSet};

use super::{LoadedRecipe, Recipe, RecipeError};
use crate::child::values::PARENT_PREFIX;
use crate::child::{ParentContext, child_frame, describe_frame, standalone_frame};
use crate::constraints::is_known_path;
use crate::kind;
use crate::render::{self, Role};
use crate::stage::{LayerSpec, MapKind, ParamError, Registry, Runs, StageParams};
use crate::world::{LayerKind, MapFrame};

/// The smallest cell edge of a map, m: a centimetre, below any floor plan's need.
pub const MIN_CELL_M: f64 = 0.01;

/// The largest cell edge of a map, m: 100 km, beyond any world map's need.
pub const MAX_CELL_M: f64 = 100_000.0;

mod offered;

/// What a plan is made for: it decides the frame, which stages take part and which parent
/// values are offered (spec P29 section 5.2 items 4 and 5).
#[derive(Debug, Clone, Copy)]
pub enum Planning<'a> {
    /// A run without a parent: stages that run only for a child are left out.
    Standalone,
    /// A run from the given parent: stages that run only without a parent are left out, and
    /// the parent's values are offered.
    Child(&'a ParentContext),
    /// `describe`: every stage is kept and every `parent.*` read counts as available.
    Describe,
}

/// The parent package and feature a child plan was made from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParentRef {
    /// SHA-256 of the parent file.
    pub package_sha256: String,
    /// The id of the feature the child details.
    pub feature: String,
}

/// A validated recipe: the frame and the stages with their parsed parameters.
#[derive(Debug)]
pub struct Plan {
    /// The recipe id.
    pub recipe_id: String,
    /// The kind of map.
    pub kind: MapKind,
    /// The frame the recipe's grid and size give.
    pub frame: MapFrame,
    /// Where the frame lies in the parent frame, m; `None` without a parent.
    pub origin_m: Option<[f64; 2]>,
    /// The parent package and feature of a child plan.
    pub parent: Option<ParentRef>,
    /// The values the runner seeds before the first stage: those that some stage reads, by name.
    pub seeded: Vec<LayerSpec>,
    /// The recipe the plan was made from; kinds build their own seeded values from it.
    pub recipe: Recipe,
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
    planning: Planning<'_>,
) -> Result<Plan, RecipeError> {
    check_id(recipe)?;
    let placed = check_frame(recipe, planning)?;
    check_kind_frame(recipe, &placed.frame)?;
    check_renders(recipe)?;
    check_inputs(recipe, inputs)?;
    let (stages, seeded) = plan_stages(recipe, registry, planning, &placed.frame)?;
    check_kind_plan(recipe, &stages, &placed.frame)?;
    check_constraints(recipe, &stages)?;
    let r = &recipe.recipe;
    Ok(Plan {
        recipe_id: r.id.clone(),
        kind: r.kind,
        frame: placed.frame,
        origin_m: placed.origin_m,
        parent: placed.parent,
        seeded,
        recipe: r.clone(),
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

/// The frame check's result: where and how big the map is.
struct Placed {
    frame: MapFrame,
    origin_m: Option<[f64; 2]>,
    parent: Option<ParentRef>,
}

/// Check 1: the frame. A map without a parent takes it from the recipe (`standalone_frame`), a
/// child from its place in the parent (`child_frame`), `describe` from the first parent type.
fn check_frame(recipe: &LoadedRecipe, planning: Planning<'_>) -> Result<Placed, RecipeError> {
    let (kind, r) = (recipe.recipe.kind, &recipe.recipe);
    Ok(match planning {
        Planning::Standalone => Placed {
            frame: standalone_frame(kind, r)?,
            origin_m: None,
            parent: None,
        },
        Planning::Describe => Placed {
            frame: describe_frame(kind, r)?,
            origin_m: None,
            parent: None,
        },
        Planning::Child(parent) => {
            let child = child_frame(kind, r, parent)?;
            Placed {
                frame: MapFrame {
                    rows: child.rows,
                    cols: child.cols,
                    cell_m: child.cell_m,
                },
                origin_m: Some(child.origin_m),
                parent: Some(ParentRef {
                    package_sha256: parent.sha256.clone(),
                    feature: parent.feature.id.clone(),
                }),
            }
        }
    })
}

/// Check 1, the kind's part: the frame keeps the rule of the recipe's kind (an interior's cell
/// size lies on the 1/16 m lattice). The refusal names `size_km`, which sets the cell size with
/// `grid`.
fn check_kind_frame(recipe: &LoadedRecipe, frame: &MapFrame) -> Result<(), RecipeError> {
    match kind::table(recipe.recipe.kind).check_frame {
        Some(check) => {
            check(frame).map_err(|message| invalid(recipe, &at_parameter("size_km"), message))
        }
        None => Ok(()),
    }
}

/// Check 4, the kind's part: the planned stages' parameters fit together under the rule of the
/// recipe's kind (an interior's walls fit its lattice, ruling R22). The refusal names the stage
/// and the parameter the kind blames.
fn check_kind_plan(
    recipe: &LoadedRecipe,
    stages: &[PlannedStage],
    frame: &MapFrame,
) -> Result<(), RecipeError> {
    let Some(check) = kind::table(recipe.recipe.kind).check_plan else {
        return Ok(());
    };
    check(stages, frame).map_err(|e| {
        invalid(
            recipe,
            &Spot {
                stage: Some(&e.stage),
                layer: None,
                parameter: e.parameter.as_deref(),
            },
            e.message.clone(),
        )
    })
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

/// Check 4: the stages, in order, with the layers available before each; each stage's
/// parameters are also checked against the `frame`.
///
/// A stage that does not run in this planning is parsed (so a wrong table is still refused) and
/// then left out: it is not counted for the random-stream keys and writes nothing. Returns the
/// stages and the offered values that some stage reads.
fn plan_stages(
    recipe: &LoadedRecipe,
    registry: &Registry,
    planning: Planning<'_>,
    frame: &MapFrame,
) -> Result<(Vec<PlannedStage>, Vec<LayerSpec>), RecipeError> {
    let offer = offered::Offered::of(&recipe.recipe, planning);
    let mut available: BTreeMap<String, LayerKind> = offer.names.clone();
    let mut seeded: BTreeMap<&'static str, LayerSpec> = BTreeMap::new();
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
        stage
            .check_frame(params.as_ref(), frame)
            .map_err(param_error)?;
        let skipped = matches!(
            (planning, stage.runs()),
            (Planning::Standalone, Runs::Child) | (Planning::Child(_), Runs::Standalone)
        );
        if skipped {
            continue;
        }
        let names: BTreeSet<&str> = available.keys().map(String::as_str).collect();
        stage
            .check_order(params.as_ref(), &names)
            .map_err(param_error)?;
        let reads = stage.reads(params.as_ref());
        for read in &reads {
            if offer.accepts(read) {
                available.entry(read.name.to_owned()).or_insert(read.kind);
            }
            if offer.names.contains_key(read.name) {
                seeded.entry(read.name).or_insert(*read);
            }
            check_read(recipe, id, read, &available)?;
        }
        let writes = stage.writes(params.as_ref());
        available.extend(
            writes
                .iter()
                .filter(|w| !w.optional)
                .map(|w| (w.name.to_owned(), w.kind)),
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
    let mut seeded: Vec<LayerSpec> = seeded.into_values().collect();
    seeded.sort_by_key(|spec| spec.name);
    Ok((stages, seeded))
}

/// Check 4, one read of stage `id`: written before, of the kind read, or optional.
fn check_read(
    recipe: &LoadedRecipe,
    id: &str,
    read: &LayerSpec,
    available: &BTreeMap<String, LayerKind>,
) -> Result<(), RecipeError> {
    let on_layer = Spot {
        stage: Some(id),
        layer: Some(read.name),
        parameter: None,
    };
    match available.get(read.name) {
        Some(kind) if *kind != read.kind => {
            let who = if read.name.starts_with(PARENT_PREFIX) {
                "the parent offers it"
            } else {
                "an earlier stage writes it"
            };
            Err(invalid(
                recipe,
                &on_layer,
                format!(
                    "stage `{id}` reads `{}` as {}, but {who} as {kind}",
                    read.name, read.kind
                ),
            ))
        }
        Some(_) => Ok(()),
        None if read.optional => Ok(()),
        None => {
            let what = if read.name.starts_with(PARENT_PREFIX) {
                "which the parent does not have"
            } else {
                "which no earlier stage writes"
            };
            Err(invalid(
                recipe,
                &on_layer,
                format!("stage `{id}` reads `{}`, {what}", read.name),
            ))
        }
    }
}

#[cfg(test)]
mod child_tests;
#[cfg(test)]
mod tests;
