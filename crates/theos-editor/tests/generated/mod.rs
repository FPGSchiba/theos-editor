//! Checked-in files that Rust generates for the UI: a test compares them byte for byte with a
//! fresh export, and `UPDATE_BINDINGS=1` writes them instead.

use std::path::{Path, PathBuf};

/// The repository root.
pub fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// Whether the run writes the files instead of comparing them.
pub fn updating() -> bool {
    std::env::var_os("UPDATE_BINDINGS").is_some_and(|value| value == "1")
}

/// Whether `checked_in`, the file's bytes or `None` when it is missing, differs from
/// `generated`.
pub fn is_stale(checked_in: Option<&[u8]>, generated: &[u8]) -> bool {
    checked_in != Some(generated)
}

/// Compares the file `relative` (to the repository root, with `/`) with `generated`, or writes
/// it when [`updating`]. `test` names the test filter of the hint in the stale message.
pub fn check(relative: &str, generated: &str, test: &str) -> Result<(), String> {
    let path = repo().join(relative);
    if updating() {
        if let Some(folder) = path.parent() {
            std::fs::create_dir_all(folder).map_err(|error| format!("{relative}: {error}"))?;
        }
        return std::fs::write(&path, generated).map_err(|error| format!("{relative}: {error}"));
    }
    let checked_in = std::fs::read(&path).ok();
    if is_stale(checked_in.as_deref(), generated.as_bytes()) {
        return Err(format!(
            "{relative} is stale: run `UPDATE_BINDINGS=1 cargo test -p theos-editor {test}`"
        ));
    }
    Ok(())
}
