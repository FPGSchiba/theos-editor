//! The layers a run's outputs read (ledger ruling P3): `run` and `describe` refuse, after `plan`
//! and before any stage runs, a recipe whose stages do not write every layer the package and
//! each of its renders read. `plan` itself stays output-agnostic, so the runner's tests can plan
//! pipelines that make no map.

use std::collections::BTreeMap;

use crate::package;
use crate::recipe::{Plan, RecipeError};
use crate::render::{self, Role};
use crate::stage::LayerSpec;
use crate::world::LayerKind;

/// The renders of `plan`: the standard ones, then the recipe's extras in its order, each once.
pub(super) fn render_names(plan: &Plan) -> Vec<&'static str> {
    let mut names: Vec<&'static str> = render::names(plan.kind, Role::Standard).collect();
    for extra in &plan.renders {
        if let Some(known) = render::names(plan.kind, Role::Extra).find(|e| *e == extra.as_str())
            && !names.contains(&known)
        {
            names.push(known);
        }
    }
    names
}

/// The layers the outputs of `plan` read, each with the output that reads it: the package's
/// first, then each render's in the order of [`render_names`].
fn needs(plan: &Plan) -> Vec<(String, LayerSpec)> {
    let mut out: Vec<(String, LayerSpec)> = package::needs(plan.kind)
        .into_iter()
        .map(|spec| ("the package".to_owned(), spec))
        .collect();
    for name in render_names(plan) {
        let specs = render::needs(plan.kind, name).unwrap_or_default();
        out.extend(
            specs
                .iter()
                .map(|&spec| (format!("the render `{name}`"), spec)),
        );
    }
    out
}

/// Checks that the stages of `plan` write every layer its outputs read, with the kind they read
/// it as. Optional writes do not count, as in `plan`'s own order check.
///
/// # Errors
/// [`RecipeError::Invalid`] naming the first layer that no stage writes, or that the stages
/// write with another kind, and the output that reads it.
pub(crate) fn check(plan: &Plan) -> Result<(), RecipeError> {
    let mut written: BTreeMap<&str, LayerKind> = BTreeMap::new();
    for stage in &plan.stages {
        let writes = stage.writes.iter().filter(|w| !w.optional);
        written.extend(writes.map(|w| (w.name, w.kind)));
    }
    for (output, need) in needs(plan) {
        let message = match written.get(need.name) {
            Some(kind) if *kind == need.kind => continue,
            Some(kind) => format!(
                "{output} reads `{}` as {}, but the stages write it as {kind}",
                need.name, need.kind
            ),
            None => format!("{output} reads `{}`, which no stage writes", need.name),
        };
        return Err(RecipeError::Invalid {
            recipe: plan.recipe_id.clone(),
            stage: None,
            layer: Some(need.name.to_owned()),
            parameter: None,
            message,
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;
    use crate::recipe::{self, RecipeOrigin, parse};
    use crate::stage::{MapKind, Registry};

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    /// The plan of the recipe `text`.
    fn planned(text: &str) -> Result<Plan, Box<dyn std::error::Error>> {
        let loaded = parse(text, RecipeOrigin::Builtin)?;
        Ok(recipe::plan(
            &loaded,
            &Registry::builtin(),
            &serde_json::Map::new(),
            recipe::Planning::Standalone,
        )?)
    }

    // Final review round 4, finding 6: both lists of renders come from `render::RENDERS`, so a
    // render is standard or extra and nothing else, in the table's order.
    #[test]
    fn the_standard_and_extra_renders_are_the_table_of_renders() {
        let standard: Vec<&str> = render::names(MapKind::Terrain, Role::Standard).collect();
        let extra: Vec<&str> = render::names(MapKind::Terrain, Role::Extra).collect();
        assert_eq!(
            standard,
            ["base", "truth", "geology", "resources", "inworld"]
        );
        assert_eq!(extra, ["omega"]);
        let all: Vec<&str> = render::RENDERS.iter().map(|r| r.name).collect();
        assert_eq!(all, [standard, extra].concat());
    }

    // Every built-in and every fixture recipe but the evolution budget fixture writes what its
    // outputs read; `evolution-384` stops after `landscape_evolution`, and tests only plan it
    // (ruling of the final review, round 1).
    #[test]
    fn the_built_in_and_fixture_recipes_pass_and_the_budget_fixture_is_refused() -> TestResult {
        let folder = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/recipes");
        let catalog = recipe::catalog(&[folder])?;
        assert!(catalog.problems.is_empty(), "{:?}", catalog.problems);
        assert!(catalog.recipes.len() >= 10, "{:?}", catalog.recipes.keys());
        for (id, loaded) in &catalog.recipes {
            let plan = recipe::plan(
                loaded,
                &Registry::builtin(),
                &serde_json::Map::new(),
                crate::recipe::Planning::Standalone,
            )?;
            let result = check(&plan);
            if id == "evolution-384" {
                match result {
                    Err(RecipeError::Invalid {
                        layer: Some(layer),
                        stage: None,
                        ..
                    }) => assert_eq!(layer, "water_surface"),
                    other => return Err(format!("{id}: expected a refusal, got {other:?}").into()),
                }
            } else {
                result.map_err(|e| format!("{id}: {e}"))?;
            }
        }
        Ok(())
    }

    #[test]
    fn a_layer_of_another_kind_is_refused() -> TestResult {
        let text = include_str!("../../tests/fixtures/recipes/api-small.toml");
        let mut plan = planned(text)?;
        let ecology = plan
            .stages
            .iter_mut()
            .find(|s| s.id == "ecology")
            .ok_or("api-small has no ecology")?;
        for write in &mut ecology.writes {
            if write.name == "vegetation" {
                *write = LayerSpec::u8("vegetation");
            }
        }
        match check(&plan) {
            Err(RecipeError::Invalid {
                layer: Some(layer),
                message,
                ..
            }) => {
                assert_eq!(layer, "vegetation");
                assert!(message.contains("as an f64 raster"), "{message}");
                assert!(message.contains("as a u8 raster"), "{message}");
            }
            other => return Err(format!("expected a refusal, got {other:?}").into()),
        }
        Ok(())
    }

    #[test]
    fn extra_renders_follow_the_standard_ones_once() -> TestResult {
        let text = include_str!("../../tests/fixtures/recipes/api-small.toml");
        let mut plan = planned(text)?;
        plan.renders.push("omega".to_owned());
        let names = render_names(&plan);
        let standard: Vec<&str> = render::names(MapKind::Terrain, Role::Standard).collect();
        assert_eq!(names[..standard.len()], *standard);
        assert_eq!(names[standard.len()..], ["omega"]);
        Ok(())
    }
}
