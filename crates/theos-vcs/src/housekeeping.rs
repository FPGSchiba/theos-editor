//! Git's own housekeeping, run on purpose after a write, in the foreground.

use std::ffi::OsStr;
use std::path::Path;

use crate::git::failed;
use crate::read_args::read_args;
use crate::{Cancel, Git, GitError};

/// Runs `git maintenance run --auto` in the foreground: git decides what is due. The write
/// options are not used, since they switch automatic housekeeping off.
///
/// # Errors
///
/// [`GitError::Cancelled`] when `cancel` is set, [`GitError::Failed`] when git fails, and
/// [`GitError::Io`] when git cannot be run.
pub fn housekeeping(git: &Git, top: &Path, cancel: &Cancel) -> Result<(), GitError> {
    let settings = [
        "-c",
        "gc.autoDetach=false",
        "-c",
        "maintenance.autoDetach=false",
    ];
    let read = read_args("maintenance", &["run", "--auto"]);
    let args: Vec<&OsStr> = settings
        .iter()
        .map(OsStr::new)
        .chain(read.iter().copied())
        .collect();
    let out = git.output(&args, top, None, &[], cancel)?;
    if out.success() {
        Ok(())
    } else {
        Err(failed(&args, out))
    }
}
