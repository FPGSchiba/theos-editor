use std::path::Path;

use serde::Deserialize;

use super::*;
use crate::api::RunRequest;
use crate::recipe::test_stages::C;
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

// AC: CE-28.4
#[test]
fn a_panicking_stage_ends_the_run_with_its_message_and_no_files() -> TestResult {
    let folder = tempfile::tempdir()?;
    std::fs::write(
        folder.path().join("boom.toml"),
        "id = \"boom\"\ntitle = \"T\"\nkind = \"terrain\"\ngrid = [4, 8]\nsize_km = [1.0, 0.5]\n\
         [[stage]]\nid = \"c\"\n[[stage]]\nid = \"boom\"\n",
    )?;
    let mut registry = Registry::empty();
    registry.add(Box::new(C));
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

#[test]
fn the_golden_record_hashes_the_written_files() -> TestResult {
    let recipes = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/recipes");
    let out = tempfile::tempdir()?;
    let (manifest, golden) = run_inner(
        &Registry::builtin(),
        &request("api-small", 1),
        &[recipes],
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
