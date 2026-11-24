//! The built-in recipes, `describe` and the refusals of a recipe before any stage runs
//! (spec §5.5, CE-28.1, CE-28.3, CE-28.7).

use std::fmt::Write as _;
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
    let ids: Vec<&str> = d.recipes.iter().map(|r| r.id.as_str()).collect();
    assert_eq!(ids, BUILT_IN, "exactly the built-ins");
    Ok(())
}

// Final review round 4, class K: the built-in recipes are the files of `recipes/`, so a new file
// there is offered and tested, and nothing is offered that the folder lacks.
#[test]
fn the_built_in_recipes_are_the_recipes_folder() -> TestResult {
    let folder = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("recipes");
    let mut files = Vec::new();
    for entry in std::fs::read_dir(folder)? {
        let name = entry?.file_name().to_string_lossy().into_owned();
        files.push(name.strip_suffix(".toml").ok_or("only recipes")?.to_owned());
    }
    files.sort();
    assert_eq!(files, BUILT_IN);
    Ok(())
}

// AC: CE-28.1
#[test]
fn a_folder_recipe_wins_over_the_built_in() -> TestResult {
    // The built-in with another title and its last stage left out: a recipe is offered only when
    // its stages write every layer of the outputs (ledger ruling P3), so it keeps the others.
    let mut table: toml::Table = include_str!("../../recipes/fixture-small.toml").parse()?;
    table.insert("title".to_owned(), "Folder fixture".into());
    let stages = table
        .get_mut("stage")
        .and_then(toml::Value::as_array_mut)
        .ok_or("fixture-small has no stages")?;
    let built_in = stages.len();
    let last = stages.pop().ok_or("fixture-small has no stages")?;
    if last.get("id").and_then(toml::Value::as_str) != Some("travel") {
        return Err(format!("fixture-small ends with {last:?}, not travel").into());
    }
    let text = toml::to_string(&table)?;
    let dir = folder_with("fixture-small.toml", &text)?;
    let d = describe(&[dir.path().to_owned()])?;
    let shown: Vec<_> = d
        .recipes
        .iter()
        .filter(|r| r.id == "fixture-small")
        .collect();
    assert_eq!(shown.len(), 1);
    assert_eq!(shown[0].title, "Folder fixture");
    assert_eq!(shown[0].stages.len(), built_in - 1);
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

/// The order of the per-step processes in a time step (spec §5.3, ruling 58): `hillslope` runs
/// before the routing, so each step ends on the surface `fluvial` cut.
const PROCESS_ORDER: [&str; 8] = [
    "uplift",
    "hillslope",
    "route_steepest",
    "resolve_sinks",
    "partition_flow",
    "fluvial",
    "slope_limit",
    "climate_update",
];

/// The process ids of the `landscape_evolution` stage of `loaded`, in the recipe's order.
fn process_ids(loaded: &recipe::LoadedRecipe) -> Vec<&str> {
    loaded
        .recipe
        .stages
        .iter()
        .filter(|s| s.get("id").and_then(toml::Value::as_str) == Some("landscape_evolution"))
        .filter_map(|s| s.get("process").and_then(toml::Value::as_array))
        .flatten()
        .filter_map(|p| p.get("id").and_then(toml::Value::as_str))
        .collect()
}

#[test]
fn every_recipe_runs_its_processes_in_the_order_of_a_time_step() -> TestResult {
    let catalog = recipe::catalog(&crate::support::recipes())?;
    assert!(catalog.problems.is_empty(), "{:?}", catalog.problems);
    for (id, loaded) in &catalog.recipes {
        let ids = process_ids(loaded);
        let places: Vec<Option<usize>> = ids
            .iter()
            .map(|p| PROCESS_ORDER.iter().position(|o| o == p))
            .collect();
        let in_order = places
            .windows(2)
            .all(|w| matches!((w[0], w[1]), (Some(a), Some(b)) if a < b));
        assert!(
            in_order && places.iter().all(Option::is_some),
            "{id}: {ids:?} is not in the order {PROCESS_ORDER:?}"
        );
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

/// A recipe whose `landscape_evolution` runs the `processes` (ids, in order) on a 64² grid.
fn evolution_recipe(id: &str, processes: &[&str]) -> String {
    let mut rest =
        format!("{TECTONICS_FIRST}\n[[stage]]\nid = \"landscape_evolution\"\nsteps = 2\n");
    for process in processes {
        let extra = if *process == "fluvial" {
            "g = 1.0\n"
        } else {
            ""
        };
        let _ = write!(rest, "[[stage.process]]\nid = \"{process}\"\n{extra}");
    }
    recipe_text(id, id, &rest)
}

/// The stages before the evolution: grid, then tectonics.
const TECTONICS_FIRST: &str = "\n[[stage]]\nid = \"grid\"\nedges = [\"south\"]\n\
                               \n[[stage]]\nid = \"tectonics\"\n";

/// The fixture `api-small` under the id `id`, without the stages `left_out`.
fn api_small_without(left_out: &[&str], id: &str) -> Result<String, Box<dyn std::error::Error>> {
    let text = std::fs::read_to_string(crate::support::recipes()[0].join("api-small.toml"))?;
    let mut table: toml::Table = text.parse()?;
    table.insert("id".to_owned(), id.into());
    let stages = table
        .get_mut("stage")
        .and_then(toml::Value::as_array_mut)
        .ok_or("api-small has no stages")?;
    let before = stages.len();
    stages.retain(|s| {
        s.get("id")
            .and_then(toml::Value::as_str)
            .is_none_or(|stage| !left_out.contains(&stage))
    });
    if stages.len() + left_out.len() != before {
        return Err(format!("api-small does not hold every stage of {left_out:?}").into());
    }
    Ok(toml::to_string(&table)?)
}

/// The layer, the message and whether the progress closure was called, for a run that failed
/// with `Invalid` naming no stage.
fn refused_layer(
    outcome: (Result<(), RunError>, bool),
) -> Result<(Option<String>, String, bool), String> {
    match outcome {
        (
            Err(RunError::Recipe(RecipeError::Invalid {
                stage: None,
                layer,
                message,
                ..
            })),
            called,
        ) => Ok((layer, message, called)),
        (other, _) => Err(format!("expected Invalid without a stage, got {other:?}")),
    }
}

// Ledger ruling P3: the extra render `omega` reads the layer `omega`, which only `detuning`
// writes; the run is refused after `plan`, before any stage runs, and `describe` reports it.
#[test]
fn an_extra_render_without_its_layer_fails_before_any_stage_runs() -> TestResult {
    let text = api_small_without(&["detuning"], "no-detuning")?;
    let dir = folder_with("no-detuning.toml", &text)?;
    let (layer, message, called) =
        refused_layer(run_from(&request("no-detuning", 1), dir.path())?)?;
    assert_eq!(layer.as_deref(), Some("omega"));
    assert!(message.contains("render `omega`"), "{message}");
    assert!(!called, "a stage ran");

    let d = describe(&[dir.path().to_owned()])?;
    assert!(d.recipes.iter().all(|r| r.id != "no-detuning"));
    assert_eq!(d.problems.len(), 1, "{:?}", d.problems);
    assert_eq!(d.problems[0].id.as_deref(), Some("no-detuning"));
    assert!(
        d.problems[0].message.contains("`omega`"),
        "{:?}",
        d.problems
    );
    Ok(())
}

// Ledger ruling P3: the package holds `element_water`, which only `elements` writes; no later
// stage is left to read it, so `plan` alone would not refuse the recipe.
#[test]
fn a_package_layer_no_stage_writes_fails_before_any_stage_runs() -> TestResult {
    let after_ecology = [
        "elements",
        "settlements",
        "pois",
        "detuning",
        "resources",
        "roads",
        "sites",
        "travel",
    ];
    let text = api_small_without(&after_ecology, "no-elements")?;
    let dir = folder_with("no-elements.toml", &text)?;
    let (layer, message, called) =
        refused_layer(run_from(&request("no-elements", 1), dir.path())?)?;
    assert_eq!(layer.as_deref(), Some("element_water"));
    assert!(message.contains("the package"), "{message}");
    assert!(!called, "a stage ran");
    Ok(())
}

// Ledger ruling P3 with the Task 42 ruling: `roads` is all zeros without the `roads` stage, so a
// recipe without roads (and so without `sites`, which reads them) still describes; `travel`
// stays, as it reads `road_network` only when present.
#[test]
fn a_recipe_without_roads_passes_the_output_check() -> TestResult {
    let text = api_small_without(&["roads", "sites"], "no-roads")?;
    let dir = folder_with("no-roads.toml", &text)?;
    let d = describe(&[dir.path().to_owned()])?;
    assert!(d.problems.is_empty(), "{:?}", d.problems);
    assert!(d.recipes.iter().any(|r| r.id == "no-roads"));
    Ok(())
}

// Spec ruling 58: `hillslope` runs before the routing of its step; a process list with it after
// a routing process or `fluvial` fails before any stage runs.
#[test]
fn a_hillslope_after_the_routing_fails_before_any_stage_runs() -> TestResult {
    for order in [
        ["uplift", "route_steepest", "hillslope"],
        ["uplift", "fluvial", "hillslope"],
    ] {
        let text = evolution_recipe("old-order", &order);
        let dir = folder_with("old-order.toml", &text)?;
        let (stage, parameter, message) = invalid(run_from(&request("old-order", 1), dir.path())?)?;
        assert_eq!(stage.as_deref(), Some("landscape_evolution"), "{order:?}");
        assert_eq!(
            parameter.as_deref(),
            Some("process[2]"),
            "{order:?}: {message}"
        );
        assert!(message.contains("`hillslope`"), "{message}");
    }
    Ok(())
}
