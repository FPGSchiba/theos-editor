//! `theos formalism`: regenerate the Shared Formalism mirror.

use std::io::Write;
use std::path::Path;

use anyhow::Context;
use theos_studio::views::ledger::FormalismView;

use crate::output;

/// Regenerates the mirror of the project at `folder` (the nearest project at or above the current
/// folder by default, see [`output::project_root`]) and writes each path it wrote to `out`, or
/// "nothing to write"; with `json`, one JSON document instead.
///
/// # Errors
///
/// Fails when there is no project, the source is missing or invalid, or a file or `out` cannot
/// be written.
pub(crate) fn run(folder: Option<&Path>, json: bool, out: &mut impl Write) -> anyhow::Result<()> {
    let root = output::project_root(folder)?;
    let regenerated = theos_content::formalism::regenerate(&root)?;
    if json {
        return output::json(out, &FormalismView::new(&regenerated.written));
    }
    report(&regenerated.written, out).context("cannot write the output")
}

fn report(written: &[String], out: &mut impl Write) -> std::io::Result<()> {
    if written.is_empty() {
        return writeln!(out, "nothing to write");
    }
    for path in written {
        writeln!(out, "{path}")?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn text(written: &[&str]) -> String {
        let written: Vec<String> = written.iter().map(|path| (*path).to_owned()).collect();
        let mut out = Vec::new();
        report(&written, &mut out).unwrap();
        String::from_utf8(out).unwrap()
    }

    #[test]
    fn report_lists_each_path() {
        assert_eq!(text(&["a/E1.md", "a/E2.md"]), "a/E1.md\na/E2.md\n");
    }

    #[test]
    fn report_says_when_there_is_nothing_to_write() {
        assert_eq!(text(&[]), "nothing to write\n");
    }
}
