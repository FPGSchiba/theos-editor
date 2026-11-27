//! `describe`: what the generator offers (spec §5.12, `docs/generators.md` §3).

use std::path::PathBuf;

use super::needs;
use super::{Description, GENERATOR, RecipeInfo, StageInfo, VERSION};
use crate::recipe::{self, LoadedRecipe, RecipeError, RecipeOrigin, RecipeProblem};
use crate::stage::Registry;

/// Describes the generator: its name and version, the recipes of the built-ins and
/// `recipe_dirs` (later folders win by id), and the recipe files that could not be loaded.
///
/// Every recipe is planned with empty inputs. One that does not plan, or whose stages do not
/// write a layer the package or a render reads (ledger ruling P3), is reported as a problem
/// naming its file and is not offered. `inputs` are the declarations of the recipe
/// (`Inputs::describe`) and every recipe is `seeded`.
///
/// # Errors
/// [`RecipeError::Io`] when a recipe folder or file cannot be read.
pub fn describe(recipe_dirs: &[PathBuf]) -> Result<Description, RecipeError> {
    let registry = Registry::builtin();
    let catalog = recipe::catalog(recipe_dirs)?;
    let mut problems = catalog.problems;
    let mut recipes = Vec::new();
    for loaded in catalog.recipes.values() {
        let planned = recipe::plan(loaded, &registry, &serde_json::Map::new())
            .and_then(|plan| needs::check(&plan).map(|()| plan));
        match planned {
            Ok(plan) => recipes.push(info(loaded, &plan, &registry)),
            Err(error) => problems.push(RecipeProblem {
                file: origin_file(loaded),
                id: Some(loaded.recipe.id.clone()),
                message: error.to_string(),
            }),
        }
    }
    Ok(Description {
        name: GENERATOR.to_owned(),
        version: VERSION.to_owned(),
        produces: vec!["map".to_owned()],
        accepts_notes: false,
        recipes,
        problems,
    })
}

/// One recipe as the description lists it.
fn info(loaded: &LoadedRecipe, plan: &recipe::Plan, registry: &Registry) -> RecipeInfo {
    let stages = plan
        .stages
        .iter()
        .map(|stage| StageInfo {
            id: stage.id.to_owned(),
            summary: registry
                .get(stage.id)
                .map_or_else(String::new, |s| s.summary().to_owned()),
        })
        .collect();
    RecipeInfo {
        id: loaded.recipe.id.clone(),
        title: loaded.recipe.title.clone(),
        inputs: loaded.recipe.inputs().describe(),
        seeded: true,
        stages,
    }
}

/// The file of `loaded`: its path, or `builtin:<id>` for an embedded recipe.
fn origin_file(loaded: &LoadedRecipe) -> PathBuf {
    match &loaded.origin {
        RecipeOrigin::File(path) => path.clone(),
        RecipeOrigin::Builtin => PathBuf::from(format!("builtin:{}", loaded.recipe.id)),
    }
}
