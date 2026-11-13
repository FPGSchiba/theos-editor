//! `theos check`: the findings of a project, or of what git would commit.

use std::io::Write;
use std::path::PathBuf;

use anyhow::Context;
use theos_studio::views::check::{CheckView, StagedView};
use theos_studio::{Binaries, binaries_folder, check, check_staged};
use theos_vcs::Git;

use crate::{Exit, output};

/// What `theos check` was asked for.
pub(crate) struct Options {
    /// The project folder; the project above the current folder by default.
    pub(crate) folder: Option<PathBuf>,
    /// Judge the staged snapshot instead of the working tree.
    pub(crate) staged: bool,
    /// The binaries folder, overriding the user settings.
    pub(crate) binaries: Option<PathBuf>,
    /// Print one JSON document instead of finding lines.
    pub(crate) json: bool,
}

/// Runs the check and prints its result.
///
/// # Errors
///
/// Fails when there is no project, the index or git fails, or `out` cannot be written.
pub(crate) fn run(
    options: Options,
    out: &mut impl Write,
    err: &mut impl Write,
) -> anyhow::Result<Exit> {
    let mut workspace = output::workspace(options.folder.as_deref())?;
    output::in_memory_note(err, &workspace).context("cannot write the output")?;
    if options.staged {
        let staged = check_staged(&mut workspace, &Git::find()?)?;
        let blocks = staged.blocks();
        if options.json {
            output::json(out, &StagedView::from(&staged))?;
        } else {
            output::findings(out, &staged.findings).context("cannot write the output")?;
            let line = if blocks {
                format!(
                    "commit refused: {} in the staged snapshot",
                    output::plural(staged.errors(), "error")
                )
            } else {
                format!(
                    "staged snapshot: no errors, {}",
                    output::plural(staged.warnings(), "warning")
                )
            };
            writeln!(err, "{line}").context("cannot write the output")?;
        }
        return Ok(exit_of(blocks));
    }
    let checked = check(&mut workspace, binaries_folder(options.binaries))?;
    let view = CheckView::from(&checked);
    let (errors, warnings) = theos_studio::check::counts(&checked.findings);
    if options.json {
        output::json(out, &view)?;
    } else {
        output::findings(out, &checked.findings).context("cannot write the output")?;
        writeln!(err, "{}", output::summary(errors, warnings))
            .context("cannot write the output")?;
    }
    if let Binaries::NotChecked { reason } = &checked.binaries {
        writeln!(err, "binaries not checked: {reason}").context("cannot write the output")?;
    }
    Ok(exit_of(errors > 0))
}

fn exit_of(blocks: bool) -> Exit {
    if blocks { Exit::Errors } else { Exit::Ok }
}
