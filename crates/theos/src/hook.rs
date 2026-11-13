//! `theos hook install`: git's pre-commit hook for the project (spec §5.8).

use std::io::Write;
use std::path::Path;

use anyhow::Context;
use theos_studio::find_root;
use theos_studio::hook::install_hook;
use theos_studio::views::hook::HookView;
use theos_vcs::Git;

use crate::{Exit, output};

/// Installs the hook of the project at `folder` (the project above the current folder by
/// default) and prints its path, plus the backup it kept, or one JSON document.
///
/// # Errors
///
/// Fails when there is no project or repository, the hook is refused, or a file or `out` cannot
/// be written.
pub(crate) fn run(
    folder: Option<&Path>,
    force: bool,
    json: bool,
    out: &mut impl Write,
) -> anyhow::Result<Exit> {
    let start = match folder {
        Some(folder) => folder.to_path_buf(),
        None => std::env::current_dir().context("no current folder")?,
    };
    let root = find_root(&start)?;
    let theos = std::env::current_exe().context("cannot find this program")?;
    let installed = install_hook(&root, &Git::find()?, &theos, force)?;
    if json {
        output::json(out, &HookView::from(&installed))?;
    } else {
        writeln!(out, "{}", installed.hook.display()).context("cannot write the output")?;
        if let Some(backup) = &installed.backup {
            writeln!(out, "kept the earlier hook as {}", backup.display())
                .context("cannot write the output")?;
        }
    }
    Ok(Exit::Ok)
}
