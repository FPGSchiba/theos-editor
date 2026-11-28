//! The checked commit: the staged snapshot is judged before git commits it.

use theos_vcs::commit::{CommitRefused, Trailer};
use theos_vcs::staged::{self, Status};
use theos_vcs::{CommitId, Git, GitError, merge};

use crate::StudioError;
use crate::lock::LockError;
use crate::staged::{StagedCheck, check_staged};
use crate::workspace::Workspace;

/// A commit that was made.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Committed {
    /// The new commit.
    pub id: CommitId,
    /// The check of the snapshot it holds: warnings only, an error blocks the commit.
    pub check: StagedCheck,
}

/// Why no commit was made.
#[non_exhaustive]
#[derive(Debug, thiserror::Error)]
pub enum CommitError {
    /// The staged snapshot has errors; nothing was committed.
    #[error("the staged snapshot has {} error(s); nothing was committed", .0.errors())]
    Blocked(StagedCheck),
    /// Nothing is staged and no merge is waiting to be concluded.
    #[error("nothing is staged")]
    NothingStaged,
    /// The message is empty, a trailer is invalid, or no comment character is free.
    #[error("{0}")]
    Message(String),
    /// Records or files (repository paths) with unresolved merge conflicts.
    #[error("unresolved merge conflicts: {}", .0.join(", "))]
    Conflicted(Vec<String>),
    /// The repo lock could not be taken.
    #[error(transparent)]
    Lock(#[from] LockError),
    /// git failed, with its own message: a hook, signing, `index.lock`.
    #[error(transparent)]
    Git(#[from] GitError),
    /// The check or the repository lookup failed.
    #[error(transparent)]
    Studio(StudioError),
}

impl From<StudioError> for CommitError {
    /// A lock failure stays a lock failure; every other studio error is kept whole.
    fn from(error: StudioError) -> CommitError {
        match error {
            StudioError::Lock(lock) => CommitError::Lock(lock),
            other => CommitError::Studio(other),
        }
    }
}

impl From<CommitRefused> for CommitError {
    fn from(refused: CommitRefused) -> CommitError {
        match refused {
            CommitRefused::Message(text) | CommitRefused::Trailer(text) => {
                CommitError::Message(text)
            }
            CommitRefused::Git(error) => CommitError::Git(error),
            other => CommitError::Message(other.to_string()),
        }
    }
}

/// Commits the staged snapshot with `message` and `trailers`, when it has no error: warnings do
/// not block. The repo lock is held throughout, and git starts only after the check has ended
/// its index transactions. The hooks run, so the `pre-commit` hook judges the snapshot once
/// more. A merge in progress is concluded by the commit.
///
/// # Errors
///
/// [`CommitError::Lock`] when the lock is busy, [`CommitError::Message`] for an empty message,
/// [`CommitError::NothingStaged`], [`CommitError::Conflicted`], [`CommitError::Blocked`] with the
/// whole check, and [`CommitError::Git`] when git or a hook refuses.
pub fn commit(
    workspace: &mut Workspace,
    git: &Git,
    message: &str,
    trailers: &[Trailer],
) -> Result<Committed, CommitError> {
    let mut held = workspace.hold("commit")?;
    if message.trim().is_empty() {
        return Err(CommitError::Message("the message is empty".to_owned()));
    }
    let root = held.project().root().to_path_buf();
    let snapshot = staged::read(git, &root)?;
    let conflicts: Vec<String> = snapshot
        .changed
        .iter()
        .filter(|entry| entry.status == Status::Unmerged)
        .map(|entry| String::from_utf8_lossy(&entry.path).into_owned())
        .collect();
    if !conflicts.is_empty() {
        return Err(CommitError::Conflicted(conflicts));
    }
    if snapshot.changed.is_empty() && merge::merge_state(git, &snapshot.top)?.is_none() {
        return Err(CommitError::NothingStaged);
    }
    let check = check_staged(&mut held, git)?;
    if check.blocks() {
        return Err(CommitError::Blocked(check));
    }
    let id = theos_vcs::commit::commit(git, &snapshot.top, message, trailers)?;
    Ok(Committed { id, check })
}
