//! `push`: the current commit to a branch of a remote, with git's message kept whole.

mod porcelain;

use std::path::Path;

use crate::read_args::read_args;
use crate::remote::remote_write;
use crate::{Cancel, CommitId, Git, GitError, repo};
use porcelain::RefLine;

/// Where to push `HEAD`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PushTarget {
    /// The remote's name or URL.
    pub remote: String,
    /// The branch on the remote, without `refs/heads/`.
    pub branch: String,
    /// Whether to record the remote branch as the upstream of the current branch.
    pub set_upstream: bool,
}

/// What a successful push did.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Pushed {
    /// The remote already had the commit.
    UpToDate,
    /// The remote branch moved.
    Updated {
        /// The old tip, `None` for a new branch.
        from: Option<CommitId>,
        /// The new tip.
        to: CommitId,
    },
}

/// Why the remote did not take the branch.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum Rejection {
    /// The remote branch has commits the local one lacks.
    NotFastForward,
    /// The remote refused it, for instance a hook.
    RemoteRefused,
    /// Another reason.
    Other,
}

/// Why a push did not succeed.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum PushError {
    /// The remote rejected the branch; `message` is git's stderr whole.
    #[error("push rejected: {message}")]
    Rejected {
        /// Why.
        reason: Rejection,
        /// Git's stderr.
        message: String,
    },
    /// Git failed before a ref was judged (no such remote, no credential); `message` is git's
    /// stderr.
    #[error("push failed: {message}")]
    Failed {
        /// Git's stderr.
        message: String,
    },
    /// Git could not be run, or was cancelled.
    #[error(transparent)]
    Git(#[from] GitError),
}

/// Resolves an abbreviated commit id of the local repository.
fn resolve(git: &Git, top: &Path, abbreviated: &str) -> Result<CommitId, GitError> {
    let spec = format!("{abbreviated}^{{commit}}");
    let args = read_args("rev-parse", &["--verify", "-q", &spec]);
    let out = git.run(&args, top, None, &[])?;
    Ok(CommitId(String::from_utf8_lossy(&out).trim().to_owned()))
}

/// Pushes `HEAD` to `refs/heads/<branch>` of the remote. Credentials come only from git's
/// credential helper; git never prompts.
///
/// # Errors
///
/// [`PushError::Rejected`], [`PushError::Failed`], and [`PushError::Git`] when git cannot be run
/// or `cancel` is set.
pub fn push(
    git: &Git,
    top: &Path,
    target: &PushTarget,
    cancel: &Cancel,
) -> Result<Pushed, PushError> {
    let refspec = format!("HEAD:refs/heads/{}", target.branch);
    let mut flags = vec!["--porcelain"];
    if target.set_upstream {
        flags.push("--set-upstream");
    }
    flags.push(&target.remote);
    flags.push(&refspec);
    let out = remote_write(git, top, "push", &flags, cancel)?;
    let line = porcelain::ref_line(&String::from_utf8_lossy(&out.stdout), &target.branch);
    match (out.success(), line) {
        (_, Some(RefLine::Rejected(reason))) => Err(PushError::Rejected {
            reason,
            message: out.stderr,
        }),
        (true, Some(RefLine::UpToDate)) => Ok(Pushed::UpToDate),
        (true, Some(RefLine::Updated { from, to })) => {
            let to = match to {
                Some(abbreviated) => resolve(git, top, &abbreviated)?,
                None => repo::head(git, top)?.ok_or_else(|| PushError::Failed {
                    message: "HEAD names no commit".to_owned(),
                })?,
            };
            let from = from.map(|id| resolve(git, top, &id)).transpose()?;
            Ok(Pushed::Updated { from, to })
        }
        _ => Err(PushError::Failed {
            message: out.stderr,
        }),
    }
}
