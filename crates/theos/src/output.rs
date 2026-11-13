//! What every command prints and how: finding lines, one JSON document, the summary and notes.

use std::io::{self, Write};
use std::path::Path;

use anyhow::Context;
use serde::Serialize;
use theos_content::Finding;
use theos_studio::{StudioError, Workspace};

/// The workspace of a command: the project in `folder`, else the one above the current folder.
///
/// # Errors
///
/// Fails when there is no project, or when it cannot be opened.
pub(crate) fn workspace(folder: Option<&Path>) -> Result<Workspace, StudioError> {
    if let Some(folder) = folder {
        return Workspace::open(folder);
    }
    let current = std::env::current_dir().map_err(|source| StudioError::Io {
        path: Path::new(".").to_path_buf(),
        source,
    })?;
    Workspace::find(&current)
}

/// Writes one line per finding.
///
/// # Errors
///
/// Fails when `out` cannot be written.
pub(crate) fn findings(out: &mut impl Write, findings: &[Finding]) -> io::Result<()> {
    for finding in findings {
        writeln!(out, "{finding}")?;
    }
    Ok(())
}

/// Writes `view` as one pretty JSON document and a newline.
///
/// # Errors
///
/// Fails when `view` cannot be serialized or `out` cannot be written.
pub(crate) fn json(out: &mut impl Write, view: &impl Serialize) -> anyhow::Result<()> {
    serde_json::to_writer_pretty(&mut *out, view).context("cannot write the output")?;
    writeln!(out).context("cannot write the output")
}

/// The counts as text: `1 error, 2 warnings`.
pub(crate) fn summary(errors: usize, warnings: usize) -> String {
    format!(
        "{}, {}",
        plural(errors, "error"),
        plural(warnings, "warning")
    )
}

/// `1 error`, `2 errors`.
pub(crate) fn plural(count: usize, noun: &str) -> String {
    let s = if count == 1 { "" } else { "s" };
    format!("{count} {noun}{s}")
}

/// Says on `err` that the index lives in memory, when it does.
///
/// # Errors
///
/// Fails when `err` cannot be written.
pub(crate) fn in_memory_note(err: &mut impl Write, workspace: &Workspace) -> io::Result<()> {
    if workspace.is_in_memory() {
        writeln!(
            err,
            "note: the index cache is in use by another process; working with an index in memory"
        )?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn summary_pluralises_each_count() {
        assert_eq!(summary(1, 2), "1 error, 2 warnings");
        assert_eq!(summary(0, 1), "0 errors, 1 warning");
    }

    #[test]
    fn json_is_pretty_and_ends_with_a_newline() -> anyhow::Result<()> {
        let mut out = Vec::new();
        json(&mut out, &vec![1, 2])?;
        assert_eq!(String::from_utf8(out)?, "[\n  1,\n  2\n]\n");
        Ok(())
    }
}
