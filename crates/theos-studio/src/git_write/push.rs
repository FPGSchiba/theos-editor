//! Push: the current branch to the remote git itself would push to, with git's message kept
//! whole. A push changes no file of the work tree or the index, so it takes no repo lock.

use std::path::Path;

use theos_vcs::push::{self, PushError, PushTarget, Pushed};
use theos_vcs::status;
use theos_vcs::{Cancel, Git, GitError, repo};

use crate::StudioError;
use crate::top::repo_top;
use crate::workspace::Workspace;

use super::pull::upstream;

/// Why a push did not happen.
#[non_exhaustive]
#[derive(Debug, thiserror::Error)]
pub enum PushFailed {
    /// `HEAD` is detached: there is no branch to push.
    #[error("HEAD is detached; switch to a branch to push")]
    Detached,
    /// No push remote is configured, and the repository has none or several remotes.
    #[error("no push remote is set and the remotes do not name one: {}", remotes.join(", "))]
    NoUpstream {
        /// The repository's remotes, sorted.
        remotes: Vec<String>,
    },
    /// The push itself failed; its message is git's own.
    #[error(transparent)]
    Push(#[from] PushError),
    /// The repository lookup or the status failed.
    #[error(transparent)]
    Studio(StudioError),
}

impl From<StudioError> for PushFailed {
    /// A git failure stays one; every other studio error is kept whole.
    fn from(error: StudioError) -> PushFailed {
        match error {
            StudioError::Git(git) => PushFailed::Push(PushError::Git(git)),
            other => PushFailed::Studio(other),
        }
    }
}

impl From<GitError> for PushFailed {
    fn from(error: GitError) -> PushFailed {
        PushFailed::Push(PushError::Git(error))
    }
}

/// Pushes the current branch to its push target (`push_target`) with `cancel`. It takes no repo
/// lock: a push changes neither the work tree nor the index.
///
/// # Errors
///
/// [`PushFailed::Detached`], [`PushFailed::NoUpstream`], and [`PushFailed::Push`] with git's own
/// message when the remote refuses or git fails, or [`GitError::Cancelled`] when `cancel` stops
/// it.
pub fn push(workspace: &mut Workspace, git: &Git, cancel: &Cancel) -> Result<Pushed, PushFailed> {
    let top = repo_top(git, workspace.project().root())?;
    let branch = status::status(git, &top, &Cancel::new())?
        .branch
        .head
        .ok_or(PushFailed::Detached)?;
    let target = push_target(git, &top, &branch)?;
    Ok(push::push(git, &top, &target, cancel)?)
}

/// The remote that setting `key` names; `.` (a branch tracking a local branch) is no remote and
/// counts as not set, so a push never lands in this repository.
fn real_remote(git: &Git, top: &Path, key: &str) -> Result<Option<String>, GitError> {
    Ok(repo::value(git, top, key)?.filter(|remote| remote != "."))
}

/// Where `branch` is pushed, in git's own order for the remote: `branch.<b>.pushRemote`, then
/// `remote.pushDefault`, then `branch.<b>.remote` (each one unless it is `.`, the local
/// repository). The branch on that remote is the upstream's
/// branch when the remote is the upstream's remote, else `branch`. With none of the three set, the
/// only remote is used and becomes the upstream.
///
/// # Errors
///
/// [`PushFailed::NoUpstream`] when none is set and the repository has zero or several remotes.
pub(crate) fn push_target(git: &Git, top: &Path, branch: &str) -> Result<PushTarget, PushFailed> {
    let upstream = upstream(git, top, branch)?;
    let remote = match real_remote(git, top, &format!("branch.{branch}.pushRemote"))? {
        Some(remote) => Some(remote),
        None => match real_remote(git, top, "remote.pushDefault")? {
            Some(remote) => Some(remote),
            None => real_remote(git, top, &format!("branch.{branch}.remote"))?,
        },
    };
    if let Some(remote) = remote {
        let branch = match upstream {
            Some((upstream_remote, name)) if upstream_remote == remote => name,
            _ => branch.to_owned(),
        };
        return Ok(PushTarget {
            remote,
            branch,
            set_upstream: false,
        });
    }
    let mut remotes = repo::remotes(git, top)?;
    if remotes.len() == 1 {
        return Ok(PushTarget {
            remote: remotes.remove(0),
            branch: branch.to_owned(),
            set_upstream: true,
        });
    }
    Err(PushFailed::NoUpstream { remotes })
}
