//! The project a command works on, and what every command prints and how: finding lines, one
//! JSON document, the summary and notes.

use std::io::{self, Write};
use std::path::{Path, PathBuf};

use anyhow::Context;
use serde::Serialize;
use theos_content::{Finding, ProjectError};
use theos_studio::{StudioError, Workspace, find_root};

/// The root of the project a command works on (spec §5.4): a `folder` given is the project root
/// and must hold `theos.toml`; without one, the nearest folder at or above the current one that
/// holds `theos.toml`.
///
/// # Errors
///
/// [`ProjectError::NotAProject`] when `folder` holds no `theos.toml`, and the errors of
/// [`find_root`] without a folder.
pub(crate) fn project_root(folder: Option<&Path>) -> Result<PathBuf, StudioError> {
    match folder {
        Some(folder) if folder.join("theos.toml").is_file() => Ok(folder.to_path_buf()),
        Some(folder) => Err(ProjectError::NotAProject {
            root: folder.to_path_buf(),
        }
        .into()),
        None => find_root(Path::new(".")),
    }
}

/// The workspace of a command: the project of [`project_root`], opened.
///
/// # Errors
///
/// Fails when there is no project, or when it cannot be opened.
pub(crate) fn workspace(folder: Option<&Path>) -> Result<Workspace, StudioError> {
    Workspace::open(&project_root(folder)?)
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

/// Rows of cells as text lines: each column padded to its widest cell, two spaces apart, and no
/// trailing spaces.
pub(crate) fn columns(rows: &[Vec<String>]) -> String {
    let count = rows.iter().map(Vec::len).max().unwrap_or(0);
    let widths: Vec<usize> = (0..count)
        .map(|column| {
            rows.iter()
                .filter_map(|row| row.get(column))
                .map(|cell| cell.chars().count())
                .max()
                .unwrap_or(0)
        })
        .collect();
    let mut text = String::new();
    for row in rows {
        let line: Vec<String> = row
            .iter()
            .zip(&widths)
            .map(|(cell, width)| format!("{cell:<width$}"))
            .collect();
        text.push_str(line.join("  ").trim_end());
        text.push('\n');
    }
    text
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

/// Says on `err` that the index lives in memory, when `in_memory`
/// ([`Workspace::is_in_memory`]). A command asks after its last workspace call, since any write
/// to the index can switch it to memory.
///
/// # Errors
///
/// Fails when `err` cannot be written.
pub(crate) fn in_memory_note(err: &mut impl Write, in_memory: bool) -> io::Result<()> {
    if in_memory {
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
    fn columns_pad_to_the_widest_cell() {
        let rows = vec![
            vec!["a".to_owned(), "bbb".to_owned(), "c".to_owned()],
            vec!["aaa".to_owned(), "b".to_owned()],
        ];
        assert_eq!(columns(&rows), "a    bbb  c\naaa  b\n");
    }

    #[test]
    fn the_in_memory_note_names_the_cause_only_in_memory() -> anyhow::Result<()> {
        let mut err = Vec::new();
        in_memory_note(&mut err, false)?;
        assert_eq!(String::from_utf8(err.clone())?, "");
        in_memory_note(&mut err, true)?;
        assert_eq!(
            String::from_utf8(err)?,
            "note: the index cache is in use by another process; working with an index in memory\n"
        );
        Ok(())
    }

    #[test]
    fn json_is_pretty_and_ends_with_a_newline() -> anyhow::Result<()> {
        let mut out = Vec::new();
        json(&mut out, &vec![1, 2])?;
        assert_eq!(String::from_utf8(out)?, "[\n  1,\n  2\n]\n");
        Ok(())
    }
}
