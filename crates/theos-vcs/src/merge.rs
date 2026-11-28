//! Merging an upstream into the current branch, leaving a clean merge uncommitted so the caller
//! can decide, and the ways out: conclude, or abort.
//!
//! The outcome is read from exit codes, `MERGE_HEAD`, the index and `HEAD`, never from git's
//! messages. Revisions go after `--end-of-options`, so a name that starts with `-` is a name.

use std::path::Path;

use crate::git::{args_text, failed};
use crate::read_args::read_args;
use crate::write_args::{checked, write, write_args};
use crate::{Cancel, CommitId, Git, GitError, repo};

/// What a merge did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Merged {
    /// The upstream is already part of `HEAD`; nothing changed.
    UpToDate,
    /// `HEAD` moved forward to the upstream's commit; no merge is pending.
    FastForward {
        /// The commit `HEAD` now is.
        to: CommitId,
    },
    /// The branches merged cleanly and the result is staged, uncommitted (`MERGE_HEAD` exists).
    Staged,
    /// Some paths conflict; the merge is pending (`MERGE_HEAD` exists, the index has unmerged
    /// entries).
    Conflicts,
}

/// A merge that is waiting to be concluded or aborted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MergeState {
    /// The commit being merged in (`MERGE_HEAD`).
    pub head: CommitId,
    /// The message git prepared for the merge commit (`MERGE_MSG`); empty when there is none.
    pub message: String,
}

/// Resolves `name` to a commit; exit 1 with no message means it does not resolve.
fn resolve(git: &Git, top: &Path, name: &str) -> Result<Option<CommitId>, GitError> {
    let args = read_args("rev-parse", &["-q", "--verify", name]);
    let out = git.output(&args, top, None, &[], &Cancel::new())?;
    if out.code == Some(1) && out.stderr.is_empty() {
        return Ok(None);
    }
    if !out.success() {
        return Err(failed(&args, out));
    }
    let text = String::from_utf8_lossy(&out.stdout);
    Ok(Some(CommitId(text.trim_end().to_owned())))
}

/// Whether the index holds unmerged entries.
fn has_unmerged(git: &Git, top: &Path) -> Result<bool, GitError> {
    let args = read_args("ls-files", &["-u", "-z"]);
    let out = git.run(&args, top, None, &[])?;
    Ok(!out.is_empty())
}

/// The error for a state none of the outcomes of a command describes.
fn unexpected(command: &str, flags: &[&str], message: &str) -> GitError {
    GitError::Output {
        args: args_text(&write_args(command, flags)),
        message: message.to_owned(),
    }
}

/// Merges `upstream` into the current branch: a fast-forward when possible, otherwise the merge
/// is staged without a commit. Git's own settings (`merge.ff`, `merge.autoStash`, ...) do not
/// change this.
///
/// # Errors
///
/// [`GitError::IndexLocked`] and [`GitError::Failed`] for any outcome that is not one of
/// [`Merged`] (an unknown revision, local changes a merge would overwrite, unrelated histories),
/// and the errors of [`Git::output`].
pub fn merge(git: &Git, top: &Path, upstream: &str) -> Result<Merged, GitError> {
    let before = repo::head(git, top)?;
    let flags = [
        "--ff",
        "--no-commit",
        "--no-edit",
        "--no-autostash",
        "--no-squash",
        "--end-of-options",
        upstream,
    ];
    let out = write(git, top, "merge", &flags, None, &Cancel::new())?;
    let pending = resolve(git, top, "MERGE_HEAD")?.is_some();
    match out.code {
        Some(0) if pending => Ok(Merged::Staged),
        Some(0) => match repo::head(git, top)? {
            Some(to) if Some(&to) != before.as_ref() => Ok(Merged::FastForward { to }),
            _ => Ok(Merged::UpToDate),
        },
        Some(1) if pending && has_unmerged(git, top)? => Ok(Merged::Conflicts),
        _ => {
            checked(&write_args("merge", &flags), out)?;
            Err(unexpected(
                "merge",
                &flags,
                "the merge ended in no known state",
            ))
        }
    }
}

/// The best common ancestor of `a` and `b`, or `None` when they share no history.
///
/// # Errors
///
/// [`GitError::Failed`] when a name is no commit, and the errors of [`Git::output`].
pub fn merge_base(git: &Git, top: &Path, a: &str, b: &str) -> Result<Option<CommitId>, GitError> {
    let args = read_args("merge-base", &["--end-of-options", a, b]);
    let out = git.output(&args, top, None, &[], &Cancel::new())?;
    // Without a common ancestor git exits with 1 and says nothing.
    if out.code == Some(1) && out.stderr.is_empty() {
        return Ok(None);
    }
    if !out.success() {
        return Err(failed(&args, out));
    }
    let text = String::from_utf8_lossy(&out.stdout);
    Ok(Some(CommitId(text.trim_end().to_owned())))
}

/// The paths (git's bytes, relative to `top`) that differ between `base` and `upstream`, renames
/// listed as a deletion and an addition.
///
/// # Errors
///
/// [`GitError::Failed`] when a revision is unknown, and the errors of [`Git::run`].
pub fn changed_since(
    git: &Git,
    top: &Path,
    base: &CommitId,
    upstream: &str,
) -> Result<Vec<Vec<u8>>, GitError> {
    let args = read_args(
        "diff",
        &[
            "--name-only",
            "-z",
            "--no-renames",
            "--end-of-options",
            &base.0,
            upstream,
            "--",
        ],
    );
    let out = git.run(&args, top, None, &[])?;
    Ok(out
        .split(|byte| *byte == 0)
        .filter(|path| !path.is_empty())
        .map(<[u8]>::to_vec)
        .collect())
}

/// The merge waiting in the repository at `top`, or `None` when there is none.
///
/// # Errors
///
/// [`GitError::Failed`] and the errors of [`Git::output`].
pub fn merge_state(git: &Git, top: &Path) -> Result<Option<MergeState>, GitError> {
    let Some(head) = resolve(git, top, "MERGE_HEAD")? else {
        return Ok(None);
    };
    let args = read_args("rev-parse", &["--git-path", "MERGE_MSG"]);
    let out = git.run(&args, top, None, &[])?;
    let printed = String::from_utf8_lossy(&out);
    // The path is relative to `top` (the folder git ran in), or absolute.
    let path = top.join(printed.trim_end_matches(['\r', '\n']));
    let message = match std::fs::read(&path) {
        Ok(bytes) => String::from_utf8_lossy(&bytes).into_owned(),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => String::new(),
        Err(source) => return Err(GitError::Io { source }),
    };
    Ok(Some(MergeState { head, message }))
}

/// Concludes the pending merge with the message git prepared. Hooks run.
///
/// # Errors
///
/// [`GitError::IndexLocked`], [`GitError::Failed`] (also when conflicts are unresolved or a hook
/// refuses), and the errors of [`Git::output`].
pub fn conclude(git: &Git, top: &Path) -> Result<CommitId, GitError> {
    let flags = ["--no-edit"];
    let out = write(git, top, "commit", &flags, None, &Cancel::new())?;
    checked(&write_args("commit", &flags), out)?;
    repo::head(git, top)?.ok_or_else(|| unexpected("commit", &flags, "HEAD does not resolve"))
}

/// Abandons the pending merge and restores the state before it.
///
/// # Errors
///
/// [`GitError::IndexLocked`], [`GitError::Failed`] (also when no merge is pending), and the
/// errors of [`Git::output`].
pub fn abort_merge(git: &Git, top: &Path) -> Result<(), GitError> {
    let flags = ["--abort"];
    let out = write(git, top, "merge", &flags, None, &Cancel::new())?;
    checked(&write_args("merge", &flags), out).map(drop)
}
