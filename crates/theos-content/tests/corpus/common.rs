//! Helpers shared by the corpus tests: finding fixture files and reading `expected.toml`.

use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

use serde::Deserialize;

/// The fixture corpus directory.
pub fn corpus() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/corpus")
}

/// Every `.md` file under `dir`, recursively, sorted; an unreadable directory is an error.
pub fn md_files(dir: &Path) -> Result<Vec<PathBuf>, Box<dyn Error>> {
    let mut files = Vec::new();
    let mut pending = vec![dir.to_path_buf()];
    while let Some(current) = pending.pop() {
        let entries = fs::read_dir(&current)
            .map_err(|error| format!("cannot read {}: {error}", current.display()))?;
        for entry in entries {
            let path = entry
                .map_err(|error| format!("cannot read an entry of {}: {error}", current.display()))?
                .path();
            if path.is_dir() {
                pending.push(path);
            } else if path.extension().is_some_and(|ext| ext == "md") {
                files.push(path);
            }
        }
    }
    files.sort();
    Ok(files)
}

/// One expectation of an `expected.toml`.
#[derive(Deserialize)]
pub struct Case {
    /// The fixture file, relative to the `expected.toml`.
    pub file: String,
    /// The rule the file must fail with.
    pub rule: Option<String>,
    /// The line of the error.
    pub line: usize,
    /// The column of the error.
    pub column: usize,
}

#[derive(Deserialize)]
struct Cases {
    case: Vec<Case>,
}

/// The `[[case]]` entries of the TOML file at `path`.
pub fn cases(path: &Path) -> Result<Vec<Case>, Box<dyn Error>> {
    let text = fs::read_to_string(path)
        .map_err(|error| format!("cannot read {}: {error}", path.display()))?;
    let parsed: Cases = toml::from_str(&text)
        .map_err(|error| format!("cannot parse {}: {error}", path.display()))?;
    Ok(parsed.case)
}
