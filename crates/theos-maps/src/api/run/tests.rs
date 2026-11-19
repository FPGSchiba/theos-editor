use std::path::Path;

use serde::Deserialize;

use super::*;
use crate::api::RunRequest;
use crate::runner::RunOptions;
use crate::stage::{LayerSpec, MapKind, StageContext, StageError, StageReport, TypedStage};

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[derive(Debug, Default, Deserialize)]
struct Knob {}

/// A stage that panics with a known message.
struct Boom;

impl TypedStage for Boom {
    type Params = Knob;
    const ID: &'static str = "boom";
    const SUMMARY: &'static str = "Panics.";
    const KINDS: &'static [MapKind] = &[MapKind::Terrain];
    fn reads(&self, _: &Knob) -> Vec<LayerSpec> {
        vec![]
    }
    fn writes(&self, _: &Knob) -> Vec<LayerSpec> {
        vec![]
    }
    fn run(&self, _: &mut StageContext<'_>, _: &Knob) -> Result<StageReport, StageError> {
        panic!("boom in the second stage")
    }
}

fn request(recipe: &str, seed: u64) -> RunRequest {
    RunRequest {
        recipe: recipe.to_owned(),
        seed,
        inputs: serde_json::Map::new(),
        from: None,
        notes: Vec::new(),
        options: RunOptions::default(),
    }
}

fn is_empty(dir: &Path) -> Result<bool, std::io::Error> {
    Ok(std::fs::read_dir(dir)?.next().is_none())
}

/// `api-small` under the id `boom` with the stage `boom` second, after `grid`: the recipe writes
/// every layer of the outputs, so it passes the check before the pipeline (ledger ruling P3).
fn api_small_with_boom() -> Result<String, Box<dyn std::error::Error>> {
    let text = std::fs::read_to_string(fixtures().join("api-small.toml"))?;
    let mut table: toml::Table = text.parse()?;
    table.insert("id".to_owned(), "boom".into());
    let stages = table
        .get_mut("stage")
        .and_then(toml::Value::as_array_mut)
        .ok_or("api-small has no stages")?;
    let boom: toml::Table = "id = \"boom\"".parse()?;
    stages.insert(1, toml::Value::Table(boom));
    Ok(toml::to_string(&table)?)
}

// AC: CE-28.4
#[test]
fn a_panicking_stage_ends_the_run_with_its_message_and_no_files() -> TestResult {
    let folder = tempfile::tempdir()?;
    std::fs::write(folder.path().join("boom.toml"), api_small_with_boom()?)?;
    let mut registry = Registry::builtin();
    registry.add(Box::new(Boom));
    let out = tempfile::tempdir()?;
    let result = run_inner(
        &registry,
        &request("boom", 1),
        &[folder.path().to_path_buf()],
        out.path(),
        &mut |_| {},
        &CancelToken::new(),
    );
    match result {
        Err(RunError::StagePanicked { stage, message }) => {
            assert_eq!(stage, "boom");
            assert!(message.contains("boom in the second stage"), "{message}");
        }
        other => return Err(format!("expected a panic, got {:?}", other.map(|_| ())).into()),
    }
    assert!(is_empty(out.path())?);
    Ok(())
}

/// The folder of the synthetic test recipes.
fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/recipes")
}

// Each step after the pipeline reports its progress event inside the step, so a progress
// closure that panics on that event panics inside the step, with no hook in the library. The
// `renders` step panics at its third render, after the features and two renders were written
// as part files; the `package` step after every render was.
// AC: CE-28.4
#[test]
fn a_panic_in_a_step_after_the_pipeline_ends_the_run_with_its_message_and_no_files() -> TestResult {
    for step in ["constraints", "features", "renders", "package"] {
        let out = tempfile::tempdir()?;
        std::fs::write(out.path().join("notes.txt"), b"keep me")?;
        let result = run_inner(
            &Registry::builtin(),
            &request("api-small", 1),
            &[fixtures()],
            out.path(),
            &mut |p| {
                let fire = p.stage == step && (step != "renders" || p.done == 2);
                assert!(!fire, "the {step} step blew up");
            },
            &CancelToken::new(),
        );
        match result {
            Err(RunError::StagePanicked { stage, message }) => {
                assert_eq!(stage, step);
                assert_eq!(message, format!("the {step} step blew up"));
            }
            other => {
                return Err(
                    format!("{step}: expected a panic, got {:?}", other.map(|_| ())).into(),
                );
            }
        }
        let left: Vec<_> = std::fs::read_dir(out.path())?
            .map(|e| e.map(|e| e.file_name()))
            .collect::<Result<_, _>>()?;
        assert_eq!(left, ["notes.txt"], "{step}");
        assert_eq!(std::fs::read(out.path().join("notes.txt"))?, b"keep me");
    }
    Ok(())
}

#[test]
fn the_golden_record_hashes_the_written_files() -> TestResult {
    let out = tempfile::tempdir()?;
    let (manifest, golden) = run_inner(
        &Registry::builtin(),
        &request("api-small", 1),
        &[fixtures()],
        out.path(),
        &mut |_| {},
        &CancelToken::new(),
    )?;
    let hash = |role: &str| -> Result<String, std::io::Error> {
        let bytes = std::fs::read(out.path().join(&manifest.outputs[role]))?;
        Ok(theos_map_format::sha256_hex(&bytes))
    };
    assert_eq!(golden.features, hash("features")?);
    assert_eq!(golden.package, hash("package")?);
    assert_eq!((golden.recipe.as_str(), golden.seed), ("api-small", 1));
    let keys: Vec<&str> = golden.stages.iter().map(|s| s.key.as_str()).collect();
    assert_eq!(keys.first(), Some(&"grid#0"));
    assert_eq!(keys.last(), Some(&"travel#0"));
    Ok(())
}
