//! `run` through the public API: progress, outputs, results, refusals, cancel and cleanup
//! (spec §5.6, §5.12).

use std::collections::BTreeSet;
use std::time::{Duration, Instant};

use theos_maps::recipe::RecipeError;
use theos_maps::{CancelToken, Progress, RunError, RunManifest, run};

use crate::support::{TestResult, files, recipes, request};

/// The stages of `api-small`, in pipeline order.
const API_SMALL_STAGES: &[&str] = &[
    "grid",
    "tectonics",
    "geology",
    "climate_model",
    "landscape_evolution",
    "hydrology",
    "ecology",
    "elements",
    "settlements",
    "pois",
    "detuning",
    "resources",
    "roads",
    "sites",
    "travel",
];

/// Runs `api-small` with `seed` into `dir` and returns the manifest and every progress event.
fn run_api_small(
    seed: u64,
    dir: &std::path::Path,
) -> Result<(RunManifest, Vec<Progress>), Box<dyn std::error::Error>> {
    let mut events = Vec::new();
    let manifest = run(
        &request("api-small", seed),
        &recipes(),
        dir,
        &mut |p| events.push(p.clone()),
        &CancelToken::new(),
    )?;
    Ok((manifest, events))
}

/// The stage of each event, with the events of per-step processes (`<stage>/<process>`) and
/// repeats of the same stage in a row left out.
fn stage_sequence(events: &[Progress]) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for event in events.iter().filter(|e| !e.stage.contains('/')) {
        if out.last() != Some(&event.stage) {
            out.push(event.stage.clone());
        }
    }
    out
}

// AC: CE-28.2
#[test]
fn run_reports_progress_and_returns_outputs_and_results() -> TestResult {
    let dir = tempfile::tempdir()?;
    let (manifest, events) = run_api_small(1, dir.path())?;

    let mut expected: Vec<String> = API_SMALL_STAGES.iter().map(|s| (*s).to_owned()).collect();
    expected.extend(
        ["constraints", "features", "renders", "package"]
            .iter()
            .map(|s| (*s).to_owned()),
    );
    assert_eq!(stage_sequence(&events), expected, "events: {events:?}");
    for (index, stage) in API_SMALL_STAGES.iter().enumerate() {
        let first = events.iter().find(|e| e.stage == *stage);
        assert_eq!(
            first.map(|e| (e.done, e.total)),
            Some((
                u32::try_from(index)?,
                u32::try_from(API_SMALL_STAGES.len())?
            )),
            "{stage}"
        );
    }

    let roles: BTreeSet<&str> = manifest.outputs.keys().map(String::as_str).collect();
    let wanted: BTreeSet<&str> = [
        "base",
        "truth",
        "inworld",
        "geology",
        "resources",
        "omega",
        "features",
        "package",
    ]
    .into_iter()
    .collect();
    assert_eq!(roles, wanted);
    let on_disk = files(dir.path())?;
    for name in manifest.outputs.values() {
        assert!(on_disk.contains_key(name), "{name} is not in {on_disk:?}");
    }
    assert_eq!(
        on_disk.len(),
        manifest.outputs.len(),
        "{:?}",
        on_disk.keys()
    );

    check_constraint_results(&manifest)?;
    assert_eq!(
        manifest.frame.size_m.map(f64::to_bits),
        [6400.0_f64.to_bits(); 2]
    );
    assert_eq!(manifest.generator, "theos-maps@1.0.0");
    assert_eq!((manifest.recipe.as_str(), manifest.seed), ("api-small", 1));
    for step in [
        "landscape_evolution",
        "constraints",
        "features",
        "renders",
        "package",
    ] {
        assert!(manifest.timing.contains_key(step), "no timing for {step}");
    }
    Ok(())
}

/// The result key of a constraint path (`docs/generators.md` §3): lower case, every other
/// character an `_`. Written here again so the test does not trust the library's own.
fn result_key(path: &str) -> String {
    path.chars()
        .map(|c| {
            let c = c.to_ascii_lowercase();
            if c.is_ascii_lowercase() || c.is_ascii_digit() {
                c
            } else {
                '_'
            }
        })
        .collect()
}

/// CE-28.2: every constraint of `api-small` is in the results under the four keys of
/// `docs/generators.md` §3 (`<key>` a number, `<key>_min` and `<key>_max` its range,
/// `<key>_ok` a boolean that says whether the value lies in it), `pass` is a boolean that says
/// whether all hold, and there are no other results (final review round 4, finding 4).
fn check_constraint_results(manifest: &RunManifest) -> TestResult {
    let recipe: toml::Table =
        std::fs::read_to_string(recipes()[0].join("api-small.toml"))?.parse()?;
    let constraints = recipe
        .get("constraints")
        .and_then(toml::Value::as_table)
        .ok_or("api-small has constraints")?;
    assert_eq!(constraints.len(), 3, "the test covers every constraint");
    let results = &manifest.results;
    let mut keys = BTreeSet::from(["pass".to_owned()]);
    let mut all = true;
    for (path, range) in constraints {
        let [min, max]: [f64; 2] = range.clone().try_into()?;
        let key = result_key(path);
        let value = results[&key].as_f64().ok_or(format!("{key} is a number"))?;
        assert_eq!(
            results[&format!("{key}_min")].as_f64(),
            Some(min),
            "{key}_min"
        );
        assert_eq!(
            results[&format!("{key}_max")].as_f64(),
            Some(max),
            "{key}_max"
        );
        let ok = results[&format!("{key}_ok")]
            .as_bool()
            .ok_or(format!("{key}_ok is a boolean"))?;
        assert_eq!(ok, min <= value && value <= max, "{key}_ok for {value}");
        all &= ok;
        keys.extend([
            key.clone(),
            format!("{key}_min"),
            format!("{key}_max"),
            format!("{key}_ok"),
        ]);
    }
    assert_eq!(results["pass"].as_bool(), Some(all), "pass");
    let found: BTreeSet<String> = results.keys().cloned().collect();
    assert_eq!(found, keys);
    Ok(())
}

#[test]
fn outputs_are_named_by_seed_and_role() -> TestResult {
    let dir = tempfile::tempdir()?;
    let (manifest, _) = run_api_small(3, dir.path())?;
    assert_eq!(manifest.outputs["base"], "s3-base.png");
    assert_eq!(manifest.outputs["omega"], "s3-omega.png");
    assert_eq!(manifest.outputs["features"], "s3-features.geojson");
    assert_eq!(manifest.outputs["package"], "s3.theosmap");
    let package = std::fs::File::open(dir.path().join("s3.theosmap"))?;
    let read = theos_map_format::read(std::io::BufReader::new(package))?;
    assert_eq!(read.manifest.recipe_id, "api-small");
    assert_eq!(
        read.manifest.results.get("pass"),
        manifest.results.get("pass")
    );
    let features: serde_json::Value =
        serde_json::from_slice(&std::fs::read(dir.path().join("s3-features.geojson"))?)?;
    assert_eq!(features["type"], "FeatureCollection");
    Ok(())
}

// map-package §5: the package's `features.geojson` holds the `features` output. Both are written
// by `theos_map_format::features_bytes`, which the writer's own tests pin to the entry's bytes.
#[test]
fn the_package_holds_the_features_output() -> TestResult {
    let dir = tempfile::tempdir()?;
    let (manifest, _) = run_api_small(2, dir.path())?;
    let output = std::fs::read(dir.path().join(&manifest.outputs["features"]))?;
    let file = std::fs::File::open(dir.path().join(&manifest.outputs["package"]))?;
    let package = theos_map_format::read(std::io::BufReader::new(file))?;
    assert_ne!(
        package.features.features.len(),
        0,
        "the run has no features"
    );
    let packaged = String::from_utf8(theos_map_format::features_bytes(&package.features)?)?;
    assert_eq!(packaged, String::from_utf8(output)?);
    Ok(())
}

#[test]
fn two_runs_into_two_folders_give_identical_outputs() -> TestResult {
    let first = tempfile::tempdir()?;
    let second = tempfile::tempdir()?;
    run_api_small(1, first.path())?;
    run_api_small(1, second.path())?;
    let a = files(first.path())?;
    let b = files(second.path())?;
    assert_eq!(a.keys().collect::<Vec<_>>(), b.keys().collect::<Vec<_>>());
    for (name, bytes) in &a {
        assert!(b[name] == *bytes, "{name} differs between the two runs");
    }
    Ok(())
}

// Final review round 3, finding 9: each stage spells its own feature types, while the summary's
// counts, the constraint paths and the render styles take theirs from `features::TYPES`; a type
// spelled otherwise in a stage would break `docs/map-package.md` §5 with every other test green.
// AC: CE-28.6
#[test]
fn every_feature_kind_and_type_is_one_of_the_types() -> TestResult {
    let dir = tempfile::tempdir()?;
    let (manifest, _) = run_api_small(1, dir.path())?;
    let bytes = std::fs::read(dir.path().join(&manifest.outputs["features"]))?;
    let collection: serde_json::Value = serde_json::from_slice(&bytes)?;
    let features = collection["features"]
        .as_array()
        .ok_or("the features output holds no feature list")?;
    let mut seen = BTreeSet::new();
    for feature in features {
        let text = |key: &str| feature["properties"][key].as_str().map(str::to_owned);
        let pair = (text("kind"), text("type"));
        let (Some(kind), Some(ty)) = pair else {
            return Err(format!("a feature without kind or type: {feature}").into());
        };
        let known = theos_maps::features::TYPES
            .iter()
            .any(|&(k, t)| k == kind && t == ty);
        assert!(known, "({kind}, {ty}) is not in features::TYPES");
        seen.insert(kind);
    }
    // The run holds every kind a terrain stage writes, so each stage's spelling was looked at.
    let kinds: BTreeSet<&str> = seen.iter().map(String::as_str).collect();
    assert_eq!(
        kinds,
        BTreeSet::from(["poi", "resource", "road", "settlement", "site", "water"])
    );
    Ok(())
}

// AC: CE-28.4
#[test]
fn cancelling_from_an_evolution_event_returns_within_one_second() -> TestResult {
    let dir = tempfile::tempdir()?;
    let token = CancelToken::new();
    let trigger = token.clone();
    let mut cancelled_at: Option<Instant> = None;
    let result = run(
        &request("hearth-valley", 1),
        &recipes(),
        dir.path(),
        &mut |p| {
            if cancelled_at.is_none() && p.stage == "landscape_evolution" && p.done >= 10 {
                trigger.cancel();
                cancelled_at = Some(Instant::now());
            }
        },
        &token,
    );
    let returned = Instant::now();
    assert!(matches!(result, Err(RunError::Cancelled)), "{result:?}");
    let at = cancelled_at.ok_or("no evolution event reached step 10")?;
    let latency = returned.duration_since(at);
    eprintln!("cancel latency at 384 squared: {latency:?}");
    assert!(latency < Duration::from_secs(1), "{latency:?}");
    assert!(files(dir.path())?.is_empty());
    Ok(())
}

// AC: CE-28.4
#[test]
fn cancelling_at_the_package_event_leaves_only_the_files_that_were_there() -> TestResult {
    let dir = tempfile::tempdir()?;
    let notes = b"notes of the user\n".to_vec();
    std::fs::write(dir.path().join("notes.txt"), &notes)?;
    let token = CancelToken::new();
    let trigger = token.clone();
    let mut seen_package = false;
    let result = run(
        &request("api-small", 1),
        &recipes(),
        dir.path(),
        &mut |p| {
            if p.stage == "package" {
                seen_package = true;
                trigger.cancel();
            }
        },
        &token,
    );
    assert!(seen_package, "no package event");
    assert!(matches!(result, Err(RunError::Cancelled)), "{result:?}");
    let left = files(dir.path())?;
    assert_eq!(left.keys().collect::<Vec<_>>(), ["notes.txt"]);
    assert_eq!(left["notes.txt"], notes);
    Ok(())
}

/// The recipe's `[render.inworld]` table reaches the render (plan ruling L9): a malformed title
/// fails `inworld`, after the features and other renders were written as part files.
#[test]
fn a_failing_render_leaves_only_the_files_that_were_there() -> TestResult {
    let folder = tempfile::tempdir()?;
    let text = std::fs::read_to_string(recipes()[0].join("api-small.toml"))?
        .replace("title  = \"Synthetic fixture\"", "title  = 5");
    std::fs::write(folder.path().join("api-small.toml"), text)?;
    let dir = tempfile::tempdir()?;
    std::fs::write(dir.path().join("notes.txt"), b"keep me")?;
    let mut steps = Vec::new();
    let result = run(
        &request("api-small", 1),
        &[folder.path().to_path_buf()],
        dir.path(),
        &mut |p| steps.push(p.stage.clone()),
        &CancelToken::new(),
    );
    match result {
        Err(RunError::Render { render, message }) => {
            assert_eq!(render, "inworld");
            assert!(message.contains("title"), "{message}");
        }
        other => return Err(format!("expected a render error, got {other:?}").into()),
    }
    let drawn = steps.iter().filter(|s| *s == "renders").count();
    assert!(drawn >= 2, "inworld was the first render: {steps:?}");
    let left = files(dir.path())?;
    assert_eq!(left.keys().collect::<Vec<_>>(), ["notes.txt"]);
    assert_eq!(left["notes.txt"], b"keep me");
    Ok(())
}

#[test]
fn existing_files_stay_and_name_clashes_are_refused() -> TestResult {
    let dir = tempfile::tempdir()?;
    std::fs::write(dir.path().join("s1-base.png"), b"not a render")?;
    std::fs::write(dir.path().join("notes.txt"), b"keep me")?;
    let before = files(dir.path())?;
    let mut called = false;
    let result = run(
        &request("api-small", 1),
        &recipes(),
        dir.path(),
        &mut |_| called = true,
        &CancelToken::new(),
    );
    match result {
        Err(RunError::OutputExists { path }) => {
            assert_eq!(path, dir.path().join("s1-base.png"));
        }
        other => return Err(format!("expected OutputExists, got {other:?}").into()),
    }
    assert!(!called, "a stage ran before the clash was found");
    assert_eq!(files(dir.path())?, before);

    let (manifest, _) = run_api_small(2, dir.path())?;
    let after = files(dir.path())?;
    for (name, bytes) in &before {
        assert_eq!(after.get(name), Some(bytes), "{name} changed");
    }
    assert_eq!(after.len(), before.len() + manifest.outputs.len());
    Ok(())
}

#[test]
fn a_leftover_part_file_is_a_clash_too() -> TestResult {
    let dir = tempfile::tempdir()?;
    std::fs::write(dir.path().join(".s1.theosmap.part"), b"a crashed run")?;
    let result = run(
        &request("api-small", 1),
        &recipes(),
        dir.path(),
        &mut |_| {},
        &CancelToken::new(),
    );
    match result {
        Err(RunError::OutputExists { path }) => {
            assert_eq!(path, dir.path().join(".s1.theosmap.part"));
        }
        other => return Err(format!("expected OutputExists, got {other:?}").into()),
    }
    assert_eq!(files(dir.path())?.len(), 1);
    Ok(())
}

#[test]
fn from_and_inputs_are_refused() -> TestResult {
    let dir = tempfile::tempdir()?;
    let mut called = false;
    let mut with_from = request("api-small", 1);
    with_from.from = Some(dir.path().join("parent.theosmap"));
    let result = run(
        &with_from,
        &recipes(),
        dir.path(),
        &mut |_| called = true,
        &CancelToken::new(),
    );
    match result {
        Err(RunError::Recipe(RecipeError::Invalid { parameter, .. })) => {
            assert_eq!(parameter.as_deref(), Some("from"));
        }
        other => return Err(format!("expected a refused `from`, got {other:?}").into()),
    }

    let mut with_inputs = request("api-small", 1);
    with_inputs
        .inputs
        .insert("floor".to_owned(), serde_json::Value::from(1));
    let result = run(
        &with_inputs,
        &recipes(),
        dir.path(),
        &mut |_| called = true,
        &CancelToken::new(),
    );
    match result {
        Err(RunError::Recipe(RecipeError::Invalid { parameter, .. })) => {
            assert_eq!(parameter.as_deref(), Some("floor"));
        }
        other => return Err(format!("expected a refused input, got {other:?}").into()),
    }
    assert!(!called);
    assert!(files(dir.path())?.is_empty());
    Ok(())
}

#[test]
fn an_unknown_recipe_is_an_error() -> TestResult {
    let dir = tempfile::tempdir()?;
    let result = run(
        &request("no-such-recipe", 1),
        &recipes(),
        dir.path(),
        &mut |_| {},
        &CancelToken::new(),
    );
    match result {
        Err(RunError::Recipe(RecipeError::UnknownRecipe(id))) => {
            assert_eq!(id, "no-such-recipe");
        }
        other => return Err(format!("expected an unknown recipe, got {other:?}").into()),
    }
    assert!(files(dir.path())?.is_empty());
    Ok(())
}

#[test]
fn a_missing_output_folder_is_an_io_error() -> TestResult {
    let dir = tempfile::tempdir()?;
    let missing = dir.path().join("not-there");
    let result = run(
        &request("api-small", 1),
        &recipes(),
        &missing,
        &mut |_| {},
        &CancelToken::new(),
    );
    match result {
        Err(RunError::Io { path, .. }) => assert_eq!(path, missing),
        other => return Err(format!("expected an I/O error, got {other:?}").into()),
    }
    assert!(!missing.exists());
    Ok(())
}

#[test]
fn the_memory_budget_comes_from_run_options() -> TestResult {
    let dir = tempfile::tempdir()?;
    // 64 KiB holds two float layers of api-small's 64² grid; its pipeline needs far more.
    let mut small = request("api-small", 1);
    small.options.memory_budget = Some(64 * 1024);
    let mut called = false;
    let result = run(
        &small,
        &recipes(),
        dir.path(),
        &mut |_| called = true,
        &CancelToken::new(),
    );
    match result {
        Err(RunError::OverBudget { budget, needed, .. }) => {
            assert_eq!(budget, 64 * 1024);
            assert!(needed > budget);
        }
        other => return Err(format!("expected OverBudget, got {other:?}").into()),
    }
    assert!(!called);
    assert!(files(dir.path())?.is_empty());
    Ok(())
}

/// Runs `api-small` with seed 1, its text changed by `edit`, and returns how the run ended.
fn run_edited_api_small(
    edit: impl Fn(String) -> String,
) -> Result<Result<RunManifest, RunError>, Box<dyn std::error::Error>> {
    let folder = tempfile::tempdir()?;
    let text = edit(std::fs::read_to_string(
        recipes()[0].join("api-small.toml"),
    )?);
    std::fs::write(folder.path().join("api-small.toml"), text)?;
    let dir = tempfile::tempdir()?;
    Ok(run(
        &request("api-small", 1),
        &[folder.path().to_path_buf()],
        dir.path(),
        &mut |_| {},
        &CancelToken::new(),
    ))
}

// Final review round 4, finding 2 (spec §5.5: a recipe `describe` accepts never panics `run`):
// numbers inside the recipe bounds that reach a render as huge pixel sizes are drawn clamped
// to the canvas; the run never ends in a panic.
#[test]
fn extreme_numbers_inside_the_bounds_never_panic_a_render() -> TestResult {
    let result = run_edited_api_small(|text| {
        text.replace(
            "min_lake_area_m2 = 30000.0",
            "min_lake_area_m2 = 30000.0\nwidth_a = 1e12",
        )
    })?;
    match result {
        Ok(manifest) => assert!(manifest.outputs.contains_key("base")),
        Err(RunError::StagePanicked { stage, message }) => {
            return Err(format!("stage `{stage}` panicked: {message}").into());
        }
        Err(other) => return Err(format!("expected a finished run, got {other}").into()),
    }
    Ok(())
}
