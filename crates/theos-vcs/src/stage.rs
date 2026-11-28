//! Staging and unstaging paths: the paths go to git NUL-separated on stdin, so their number and
//! their spelling cannot overflow or change a command line.
//!
//! Follows spec P47 §5.3.

use std::ffi::OsStr;
use std::path::Path;

use crate::write_args::{checked, write, write_args};
use crate::{Cancel, Git, GitError, repo};

/// The flags that make git read the paths from stdin, NUL-separated.
const FROM_STDIN: [&str; 2] = ["--pathspec-from-file=-", "--pathspec-file-nul"];

/// The paths NUL-joined, each one ended by a NUL.
fn nul_joined(paths: &[&[u8]]) -> Vec<u8> {
    let mut bytes = Vec::new();
    for path in paths {
        bytes.extend_from_slice(path);
        bytes.push(0);
    }
    bytes
}

/// Runs `command` with `flags`, the stdin flags and `tail` (after them), and the paths on stdin;
/// no git runs for an empty list. A write on paths is not cancellable (spec 5.2).
fn run(
    git: &Git,
    top: &Path,
    command: &str,
    flags: &[&str],
    tail: &[&str],
    paths: &[&[u8]],
) -> Result<(), GitError> {
    if paths.is_empty() {
        return Ok(());
    }
    let flags: Vec<&str> = (flags.iter().copied())
        .chain(FROM_STDIN)
        .chain(tail.iter().copied())
        .collect();
    let stdin = nul_joined(paths);
    let out = write(git, top, command, &flags, Some(&stdin), &Cancel::new())?;
    let args: Vec<&OsStr> = write_args(command, &flags);
    checked(&args, out).map(drop)
}

/// Stages the changes of `paths` (relative to `top`, git's bytes), deletions and renames
/// included. Every path is literal.
///
/// # Errors
///
/// [`GitError::IndexLocked`], [`GitError::Failed`] and the errors of [`Git::output`].
pub fn stage(git: &Git, top: &Path, paths: &[&[u8]]) -> Result<(), GitError> {
    run(git, top, "add", &["-A"], &[], paths)
}

/// Takes `paths` out of the index, keeping the working tree as it is. Before the first commit
/// they become untracked again.
///
/// # Errors
///
/// [`GitError::IndexLocked`], [`GitError::Failed`] and the errors of [`Git::output`].
pub fn unstage(git: &Git, top: &Path, paths: &[&[u8]]) -> Result<(), GitError> {
    if paths.is_empty() {
        return Ok(());
    }
    match repo::head(git, top)? {
        // `HEAD --` after the flags: a file or folder named HEAD cannot make the revision ambiguous.
        Some(_) => run(git, top, "reset", &["-q"], &["HEAD", "--"], paths),
        None => run(
            git,
            top,
            "rm",
            &["-q", "--cached", "-f", "--ignore-unmatch"],
            &[],
            paths,
        ),
    }
}
