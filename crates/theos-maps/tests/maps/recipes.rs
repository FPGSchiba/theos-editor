//! The built-in recipes, `describe` and the refusals of a recipe before any stage runs
//! (spec §5.5, CE-28.1, CE-28.3, CE-28.7).

use std::path::Path;

use theos_maps::recipe::{self, RecipeError};
use theos_maps::stage::Registry;
use theos_maps::{CancelToken, RunError, RunRequest, describe, run};

use crate::support::{TestResult, request};

/// The ids of the recipes built into the crate.
const BUILT_IN: [&str; 3] = ["fixture-small", "hearth-valley", "hearth-valley-omega"];

const GRID: &str = "\n[[stage]]\nid = \"grid\"\nedges = [\"south\"]\n";
const TECTONICS: &str = "\n[[stage]]\nid = \"tectonics\"\n";

/// A folder with the recipe file `name` holding `text`.
fn folder_with(name: &str, text: &str) -> Result<tempfile::TempDir, std::io::Error> {
    let dir = tempfile::tempdir()?;
    std::fs::write(dir.path().join(name), text)?;
    Ok(dir)
}

/// A synthetic recipe on a 64 squared grid; `rest` holds its stages and further tables.
fn recipe_text(id: &str, title: &str, rest: &str) -> String {
    format!(
        "id = \"{id}\"\ntitle = \"{title}\"\nkind = \"terrain\"\ngrid = [64, 64]\n\
         size_km = [4.0, 4.0]\n{rest}"
    )
}

/// Runs `request` with the recipes of `dir` into a fresh output folder; returns the result
/// and whether the progress closure was called.
fn run_from(
    request: &RunRequest,
    dir: &Path,
) -> Result<(Result<(), RunError>, bool), std::io::Error> {
    let out = tempfile::tempdir()?;
    let mut called = false;
    let result = run(
        request,
        &[dir.to_owned()],
        out.path(),
        &mut |_| called = true,
        &CancelToken::new(),
    )
    .map(|_| ());
    Ok((result, called))
}

/// The `Invalid` error of a run that must have failed before any stage ran.
fn invalid(
    outcome: (Result<(), RunError>, bool),
) -> Result<(Option<String>, Option<String>, String), String> {
    match outcome {
        (_, true) => Err("the progress closure was called".to_owned()),
        (
            Err(RunError::Recipe(RecipeError::Invalid {
                stage,
                parameter,
                message,
                ..
            })),
            false,
        ) => Ok((stage, parameter, message)),
        (other, _) => Err(format!("expected Invalid, got {other:?}")),
    }
}

// AC: CE-28.1
#[test]
fn describe_lists_the_three_built_in_recipes() -> TestResult {
    let d = describe(&[])?;
    assert_eq!(d.name, "theos-maps");
    assert_eq!(d.version, "1.0.0");
    assert_eq!(d.produces, ["map"]);
    assert!(!d.accepts_notes);
    assert!(d.problems.is_empty(), "{:?}", d.problems);
    for id in BUILT_IN {
        let info = d
            .recipes
            .iter()
            .find(|r| r.id == id)
            .ok_or_else(|| format!("{id} is not described"))?;
        assert!(info.inputs.is_empty(), "{id}");
        assert!(info.seeded, "{id}");
        assert!(!info.title.is_empty(), "{id}");
        assert_eq!(info.stages.first().map(|s| s.id.as_str()), Some("grid"));
        assert!(info.stages.iter().all(|s| !s.summary.is_empty()), "{id}");
    }
    Ok(())
}

// AC: CE-28.1
#[test]
fn a_folder_recipe_wins_over_the_built_in() -> TestResult {
    let text = recipe_text("fixture-small", "Folder fixture", GRID);
    let dir = folder_with("fixture-small.toml", &text)?;
    let d = describe(&[dir.path().to_owned()])?;
    let shown: Vec<_> = d
        .recipes
        .iter()
        .filter(|r| r.id == "fixture-small")
        .collect();
    assert_eq!(shown.len(), 1);
    assert_eq!(shown[0].title, "Folder fixture");
    assert_eq!(shown[0].stages.len(), 1);
    assert_eq!(d.recipes.len(), BUILT_IN.len());
    Ok(())
}

// AC: CE-28.3
#[test]
fn a_stage_before_its_input_fails_before_any_stage_runs() -> TestResult {
    let text = recipe_text("misordered", "Misordered", &format!("{TECTONICS}{GRID}"));
    let dir = folder_with("misordered.toml", &text)?;
    let (stage, _, message) = invalid(run_from(&request("misordered", 1), dir.path())?)?;
    assert_eq!(stage.as_deref(), Some("tectonics"));
    assert!(message.contains("frame"), "{message}");
    Ok(())
}

// AC: CE-28.3
#[test]
fn an_unknown_stage_fails_before_any_stage_runs() -> TestResult {
    let rest = format!("{GRID}\n[[stage]]\nid = \"erosion_magic\"\n");
    let dir = folder_with(
        "u.toml",
        &recipe_text("unknown-stage", "Unknown stage", &rest),
    )?;
    let (stage, _, _) = invalid(run_from(&request("unknown-stage", 1), dir.path())?)?;
    assert_eq!(stage.as_deref(), Some("erosion_magic"));
    Ok(())
}

// AC: CE-28.3
#[test]
fn an_unknown_parameter_fails_before_any_stage_runs() -> TestResult {
    let rest = "\n[[stage]]\nid = \"grid\"\nedges = [\"south\"]\nroughness = 3\n";
    let text = recipe_text("unknown-parameter", "Unknown parameter", rest);
    let dir = folder_with("u.toml", &text)?;
    let (stage, parameter, _) = invalid(run_from(&request("unknown-parameter", 1), dir.path())?)?;
    assert_eq!(stage.as_deref(), Some("grid"));
    assert_eq!(parameter.as_deref(), Some("roughness"));
    Ok(())
}

// AC: CE-28.3
#[test]
fn a_recipe_input_fails_as_unknown() -> TestResult {
    let declared = recipe_text(
        "declares",
        "Declares",
        &format!("{GRID}\n[inputs]\nfloor = 1\n"),
    );
    let dir = folder_with("declares.toml", &declared)?;
    let (_, parameter, message) = invalid(run_from(&request("declares", 1), dir.path())?)?;
    assert_eq!(parameter.as_deref(), Some("floor"));
    assert!(message.contains("unknown recipe input"), "{message}");

    let plain = recipe_text("plain", "Plain", GRID);
    let dir = folder_with("plain.toml", &plain)?;
    let mut given = request("plain", 1);
    given.inputs.insert("floor".to_owned(), 1.into());
    let (_, parameter, message) = invalid(run_from(&given, dir.path())?)?;
    assert_eq!(parameter.as_deref(), Some("floor"));
    assert!(message.contains("unknown recipe input"), "{message}");
    Ok(())
}

// AC: CE-28.7
#[test]
fn slow_hearth_valley_omega_writes_the_omega_layer() -> TestResult {
    let out = tempfile::tempdir()?;
    let manifest = run(
        &request("hearth-valley-omega", 1),
        &[],
        out.path(),
        &mut |_| {},
        &CancelToken::new(),
    )?;
    let file = std::fs::File::open(out.path().join(&manifest.outputs["package"]))?;
    let package = theos_map_format::read(std::io::BufReader::new(file))?;
    let entry = package
        .manifest
        .layers
        .iter()
        .find(|l| l.name == "omega")
        .ok_or("the package has no omega layer")?;
    assert_eq!(
        entry.scale.map(f64::to_bits),
        Some((1.0_f64 / 255.0).to_bits())
    );
    assert!(manifest.outputs.contains_key("omega"));
    Ok(())
}

// Review Focus 5
#[test]
fn a_broken_override_hides_the_built_in() -> TestResult {
    let dir = folder_with("fixture-small.toml", "id = \"fixture-small\"\ntitle = 7\n")?;
    let d = describe(&[dir.path().to_owned()])?;
    assert!(d.recipes.iter().all(|r| r.id != "fixture-small"));
    assert_eq!(d.problems.len(), 1, "{:?}", d.problems);
    assert!(d.problems[0].file.ends_with("fixture-small.toml"));
    assert_eq!(d.problems[0].id.as_deref(), Some("fixture-small"));
    assert_eq!(d.recipes.len(), BUILT_IN.len() - 1);

    let (result, called) = run_from(&request("fixture-small", 1), dir.path())?;
    match result {
        Err(RunError::Recipe(error @ RecipeError::Parse { .. })) => {
            let text = error.to_string();
            let file = dir.path().join("fixture-small.toml").display().to_string();
            assert!(
                text.starts_with(&format!("cannot parse recipe {file}: ")),
                "{text}"
            );
            assert_eq!(text.matches("cannot parse recipe").count(), 1, "{text}");
        }
        other => return Err(format!("expected the parse error, got {other:?}").into()),
    }
    assert!(!called);
    Ok(())
}

#[test]
fn every_built_in_recipe_plans() -> TestResult {
    let catalog = recipe::catalog(&[])?;
    assert!(catalog.problems.is_empty(), "{:?}", catalog.problems);
    let ids: Vec<&str> = catalog.recipes.keys().map(String::as_str).collect();
    assert_eq!(ids, BUILT_IN);
    let registry = Registry::builtin();
    for (id, loaded) in &catalog.recipes {
        recipe::plan(loaded, &registry, &serde_json::Map::new())
            .map_err(|e| format!("{id}: {e}"))?;
    }
    Ok(())
}

// Review Focus 5
#[test]
fn a_file_that_is_not_toml_hides_the_built_in_of_its_name() -> TestResult {
    let dir = folder_with(
        "fixture-small.toml",
        "id = 
",
    )?;
    let d = describe(&[dir.path().to_owned()])?;
    assert!(d.recipes.iter().all(|r| r.id != "fixture-small"));
    assert_eq!(d.problems.len(), 1, "{:?}", d.problems);
    assert_eq!(d.problems[0].id.as_deref(), Some("fixture-small"));

    let (result, called) = run_from(&request("fixture-small", 1), dir.path())?;
    match result {
        Err(RunError::Recipe(RecipeError::Parse { origin, .. })) => {
            assert!(origin.ends_with("fixture-small.toml"), "{origin}");
        }
        other => return Err(format!("expected the parse error, got {other:?}").into()),
    }
    assert!(!called);
    Ok(())
}
