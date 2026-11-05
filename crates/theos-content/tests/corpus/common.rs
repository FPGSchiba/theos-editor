//! Helpers shared by the corpus tests: finding fixture files and reading `expected.toml`.

use std::fs;
use std::path::{Path, PathBuf};

use serde::Deserialize;

/// The fixture corpus directory.
pub fn corpus() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/corpus")
}

/// Every `.md` file under `dir`, recursively, sorted.
pub fn md_files(dir: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    let mut pending = vec![dir.to_path_buf()];
    while let Some(current) = pending.pop() {
        let Ok(entries) = fs::read_dir(&current) else {
            continue;
        };
        for path in entries.filter_map(|entry| entry.ok().map(|e| e.path())) {
            if path.is_dir() {
                pending.push(path);
            } else if path.extension().is_some_and(|ext| ext == "md") {
                files.push(path);
            }
        }
    }
    files.sort();
    files
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
pub fn cases(path: &Path) -> Vec<Case> {
    let text = fs::read_to_string(path).unwrap_or_default();
    toml::from_str::<Cases>(&text).map_or_else(|_| Vec::new(), |parsed| parsed.case)
}
