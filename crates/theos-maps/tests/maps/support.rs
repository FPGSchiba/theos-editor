//! What the behaviour tests share: the fixture folder, requests and a look at an output folder.

use std::collections::BTreeMap;
use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};

use theos_map_format::Package;
use theos_maps::kernels::stats;
use theos_maps::{CancelToken, Progress, RunManifest, RunOptions, RunRequest, run};

/// A test's result; `?` turns any error into a failure.
pub type TestResult = Result<(), Box<dyn std::error::Error>>;

/// An error that can cross from a run's thread to the test.
pub type Error = Box<dyn std::error::Error + Send + Sync>;

/// The folder of the synthetic test recipes.
pub fn recipes() -> Vec<PathBuf> {
    vec![Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/recipes")]
}

/// A request for `recipe` and `seed` with no inputs, no `from`, no notes and default options.
pub fn request(recipe: &str, seed: u64) -> RunRequest {
    RunRequest {
        recipe: recipe.to_owned(),
        seed,
        inputs: serde_json::Map::new(),
        from: None,
        notes: Vec::new(),
        options: RunOptions::default(),
    }
}

/// Every file directly in `dir` by name, with its bytes.
pub fn files(dir: &Path) -> Result<BTreeMap<String, Vec<u8>>, std::io::Error> {
    let mut out = BTreeMap::new();
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().into_owned();
        out.insert(name, std::fs::read(entry.path())?);
    }
    Ok(out)
}

/// Runs `recipe` with `seed` (looking in `recipe_dirs` before the built-ins) into a temporary
/// folder and reads back the package it wrote, with the run's manifest.
pub fn run_package(
    recipe: &str,
    seed: u64,
    recipe_dirs: &[PathBuf],
) -> Result<(Package, RunManifest), Error> {
    let dir = tempfile::tempdir()?;
    let manifest = run(
        &request(recipe, seed),
        recipe_dirs,
        dir.path(),
        &mut |_: &Progress| {},
        &CancelToken::new(),
    )?;
    let file = manifest
        .outputs
        .get("package")
        .ok_or("the run wrote no package")?;
    let package = theos_map_format::read(BufReader::new(File::open(dir.path().join(file))?))?;
    Ok((package, manifest))
}

/// The median of `values` with numpy's semantics (`theos_maps::kernels::stats::median`); NaN when
/// there are none.
pub fn median(values: impl IntoIterator<Item = f64>) -> f64 {
    let values: Vec<f64> = values.into_iter().collect();
    stats::median(&values).unwrap_or(f64::NAN)
}
