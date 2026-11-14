//! What the behaviour tests share: the fixture folder, requests and a look at an output folder.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use theos_maps::{RunOptions, RunRequest};

/// A test's result; `?` turns any error into a failure.
pub type TestResult = Result<(), Box<dyn std::error::Error>>;

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
