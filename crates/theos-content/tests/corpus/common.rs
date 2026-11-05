//! Helpers shared by the corpus tests: finding fixture files and reading `expected.toml`.

use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

use serde::Deserialize;
use theos_content::Span;

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
    /// The kind of malformed construct the case expects.
    pub kind: Option<String>,
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

/// Every valid record of the corpus: `records/` and, once it exists, `malformed/`.
pub fn project_records() -> Result<Vec<PathBuf>, Box<dyn Error>> {
    let mut files = md_files(&corpus().join("records"))?;
    let malformed = corpus().join("malformed");
    if malformed.is_dir() {
        files.extend(md_files(&malformed)?);
    }
    Ok(files)
}

/// The text without the bytes of `span`; the whole text when there is no span.
pub fn cut(text: &str, span: Option<Span>) -> String {
    match span {
        Some(span) => format!("{}{}", &text[..span.start], &text[span.end..]),
        None => text.to_owned(),
    }
}

/// Whether `new` equals `old` in every byte before and after `region` of `old` (CE-09.2: an edit
/// changes only the bytes of one field's lines).
pub fn only_region_changed(old: &str, new: &str, region: Span) -> bool {
    let tail = old.len() - region.end;
    new.len() >= region.start + tail
        && new[..region.start] == old[..region.start]
        && new[new.len() - tail..] == old[region.end..]
}
