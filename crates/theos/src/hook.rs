//! `theos hook install`: git's `pre-commit` and `pre-merge-commit` hooks for the project (spec
//! §5.8).

use std::io::Write;
use std::path::Path;

use anyhow::Context;
use theos_studio::hook::install_hook;
use theos_studio::views::hook::HookView;
use theos_vcs::Git;

use crate::{Exit, output};

/// Installs the hooks of the project at `folder` (the project above the current folder by
/// default, see [`output::project_root`]) and prints the path of `pre-commit`, then that of
/// `pre-merge-commit`, then each backup it kept, or one JSON document.
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
    let root = output::project_root(folder)?;
    let theos = std::env::current_exe().context("cannot find this program")?;
    let installed = install_hook(&root, &Git::find()?, &theos, force)?;
    if json {
        output::json(out, &HookView::from(&installed))?;
    } else {
        let merge = &installed.pre_merge_commit;
        for hook in [&installed.hook, &merge.hook] {
            writeln!(out, "{}", hook.display()).context("cannot write the output")?;
        }
        for backup in [&installed.backup, &merge.backup].into_iter().flatten() {
            writeln!(out, "kept the earlier hook as {}", backup.display())
                .context("cannot write the output")?;
        }
    }
    Ok(Exit::Ok)
}
