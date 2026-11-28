//! Pull: fetch the upstream outside the repo lock, then merge it under the lock, judged as a
//! commit is; and the way out of a waiting merge.

use std::collections::HashSet;
use std::path::Path;

use theos_content::Index;
use theos_vcs::merge::{self, Merged};
use theos_vcs::status::{self, Change, Entry, Status};
use theos_vcs::{Cancel, CommitId, Folding, Git, GitError, remote, repo};

use crate::StudioError;
use crate::changes::{Changes, RecordChange, changes};
use crate::lock::LockError;
use crate::staged::{StagedCheck, check_staged};
use crate::top::repo_top;
use crate::workspace::Workspace;

/// What a pull did.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Pulled {
    /// The branch has every commit of its upstream; nothing changed.
    UpToDate,
    /// The branch moved forward to the upstream's commit.
    FastForward {
        /// The commit the branch now is.
        to: CommitId,
    },
    /// The upstream was merged and the merge committed.
    Merged {
        /// The merge commit.
        id: CommitId,
        /// The check of the merged result: warnings only.
        check: StagedCheck,
    },
    /// The merge stopped in conflicts; the conflicted records name them, for the merge tool.
    Conflicts(Changes),
    /// The merged result has errors: the merge waits, staged, for a fix and a commit, or an abort.
    Refused {
        /// The check of the merged result.
        check: StagedCheck,
        /// The changes the merge left, staged.
        changes: Changes,
    },
}

/// Why a pull did not merge.
#[non_exhaustive]
#[derive(Debug, thiserror::Error)]
pub enum PullError {
    /// `HEAD` is detached: there is no branch to pull into.
    #[error("HEAD is detached; switch to a branch to pull")]
    Detached,
    /// The branch has no upstream, or its upstream is gone from the remote.
    #[error("the branch `{branch}` has no upstream")]
    NoUpstream {
        /// The branch.
        branch: String,
    },
    /// Uncommitted changes the merge would overwrite; commit or discard them first. Nothing was
    /// merged.
    #[error("commit or discard these changes first: {}", named(records, files))]
    LocalChanges {
        /// The records in the way.
        records: Vec<RecordChange>,
        /// The other files in the way.
        files: Vec<Entry>,
    },
    /// A merge is waiting to be concluded or aborted.
    #[error("a merge is in progress; conclude it with a commit or abort it")]
    MergeInProgress,
    /// The repo lock could not be taken.
    #[error(transparent)]
    Lock(#[from] LockError),
    /// git failed, with its own message: the fetch, the merge, a hook.
    #[error(transparent)]
    Git(#[from] GitError),
    /// The check, the changes or the repository lookup failed.
    #[error(transparent)]
    Studio(StudioError),
}

impl From<StudioError> for PullError {
    /// A lock or git failure stays one; every other studio error is kept whole.
    fn from(error: StudioError) -> PullError {
        match error {
            StudioError::Lock(lock) => PullError::Lock(lock),
            StudioError::Git(git) => PullError::Git(git),
            other => PullError::Studio(other),
        }
    }
}

/// The records and files of [`PullError::LocalChanges`], for its message.
fn named(records: &[RecordChange], files: &[Entry]) -> String {
    let mut names: Vec<String> = records.iter().map(|record| record.id.clone()).collect();
    names.extend(
        files
            .iter()
            .flat_map(entry_paths)
            .map(|path| String::from_utf8_lossy(path).into_owned()),
    );
    names.join(", ")
}

/// Pulls the upstream of the current branch: fetches its remote with `cancel` and without the
/// repo lock, then, under the lock, fast-forwards or merges. A merged result is judged as a
/// commit is ([`check_staged`]): without errors the merge is committed, with errors it waits.
/// After the lock is released the index syncs what the pull changed.
///
/// # Errors
///
/// [`PullError::Detached`], [`PullError::NoUpstream`] (also for an upstream gone from the
/// remote), [`PullError::MergeInProgress`], [`PullError::LocalChanges`] when uncommitted changes
/// are in the merge's way (nothing is merged), [`PullError::Lock`], and [`PullError::Git`] when
/// git fails, with [`GitError::Cancelled`] when `cancel` stops the fetch.
pub fn pull(workspace: &mut Workspace, git: &Git, cancel: &Cancel) -> Result<Pulled, PullError> {
    let top = repo_top(git, workspace.project().root())?;
    let first = status::status(git, &top, &Cancel::new())?;
    let branch = first.branch.head.ok_or(PullError::Detached)?;
    let Some((remote_name, upstream_branch)) = upstream(git, &top, &branch)? else {
        return Err(PullError::NoUpstream { branch });
    };
    if merge::merge_state(git, &top)?.is_some() {
        return Err(PullError::MergeInProgress);
    }
    remote::fetch(git, &top, &remote_name, cancel)?;
    let target = tracking_ref(&remote_name, &upstream_branch);
    let mut touched = false;
    let result = merge_locked(workspace, git, &top, &branch, &target, &mut touched);
    if touched && let Err(error) = workspace.index_mut().sync() {
        tracing::warn!(%error, "the index could not sync after the pull");
    }
    result
}

/// The ref the upstream `branch` of `remote` is fetched into; a local upstream (`.`) is the
/// branch itself.
fn tracking_ref(remote: &str, branch: &str) -> String {
    if remote == "." {
        format!("refs/heads/{branch}")
    } else {
        format!("refs/remotes/{remote}/{branch}")
    }
}

/// Steps 3 to 5 of the pull, under the repo lock. `touched` is set once git started the merge.
fn merge_locked(
    workspace: &mut Workspace,
    git: &Git,
    top: &Path,
    branch: &str,
    target: &str,
    touched: &mut bool,
) -> Result<Pulled, PullError> {
    let mut held = workspace.hold("pull")?;
    let now = status::status(git, top, &Cancel::new())?;
    let Some(upstream) = now.branch.upstream.as_ref() else {
        return Err(PullError::NoUpstream {
            branch: branch.to_owned(),
        });
    };
    if upstream.behind == 0 {
        return Ok(Pulled::UpToDate);
    }
    let diverged = upstream.ahead > 0;
    in_the_way(&held, git, top, target, &now, diverged)?;
    *touched = true;
    match merge::merge(git, top, target)? {
        Merged::UpToDate => Ok(Pulled::UpToDate),
        Merged::FastForward { to } => Ok(Pulled::FastForward { to }),
        Merged::Conflicts => Ok(Pulled::Conflicts(changes(&held, git, &Cancel::new())?)),
        Merged::Staged => {
            // The check reads the index for every path the snapshot does not change, so it
            // must hold the merged working tree first.
            held.write(Index::sync)?;
            let check = check_staged(&mut held, git)?;
            if check.blocks() {
                let changes = changes(&held, git, &Cancel::new())?;
                return Ok(Pulled::Refused { check, changes });
            }
            let id = merge::conclude(git, top)?;
            Ok(Pulled::Merged { id, check })
        }
    }
}

/// Fails with [`PullError::LocalChanges`] when an uncommitted change is on a path the upstream
/// changed since the merge base, or, for a merge that is no fast-forward (`diverged`), when
/// anything is staged.
fn in_the_way(
    workspace: &Workspace,
    git: &Git,
    top: &Path,
    target: &str,
    status: &Status,
    diverged: bool,
) -> Result<(), PullError> {
    let folding = Folding::read(git, top)?;
    let upstream_changed: HashSet<Vec<u8>> = match merge::merge_base(git, top, "HEAD", target)? {
        Some(base) => merge::changed_since(git, top, &base, target)?
            .iter()
            .map(|path| key(folding, path))
            .collect(),
        // Unrelated histories: git refuses the merge itself.
        None => HashSet::new(),
    };
    let mut blocked: HashSet<Vec<u8>> = HashSet::new();
    for entry in &status.entries {
        let staged_counts = diverged && is_staged(entry);
        for path in entry_paths(entry) {
            let path = key(folding, path);
            if staged_counts || upstream_changed.contains(&path) {
                blocked.insert(path);
            }
        }
    }
    if blocked.is_empty() {
        return Ok(());
    }
    let listed = changes(workspace, git, &Cancel::new())?;
    let hit = |path: &[u8]| blocked.contains(&key(folding, path));
    let records = listed
        .records
        .into_iter()
        .filter(|record| hit(&record.repo_path) || record.repo_from.as_deref().is_some_and(hit))
        .collect();
    let files = listed
        .files
        .into_iter()
        .filter(|entry| entry_paths(entry).any(|path| hit(path)))
        .collect();
    Err(PullError::LocalChanges { records, files })
}

/// `path` as the repository compares it: folded when it is UTF-8, its bytes otherwise.
fn key(folding: Folding, path: &[u8]) -> Vec<u8> {
    match std::str::from_utf8(path) {
        Ok(text) => folding.fold(text).into_owned().into_bytes(),
        Err(_) => path.to_vec(),
    }
}

/// The repository paths of a status entry: both paths of a rename.
fn entry_paths(entry: &Entry) -> impl Iterator<Item = &Vec<u8>> {
    let (first, second) = match entry {
        Entry::Changed { path, .. }
        | Entry::Conflicted { path, .. }
        | Entry::Untracked { path } => (Some(path), None),
        Entry::Renamed { from, to, .. } => (Some(from), Some(to)),
        _ => (None, None),
    };
    first.into_iter().chain(second)
}

/// Whether the entry has a change in the index: a staged change, or a conflict.
fn is_staged(entry: &Entry) -> bool {
    match entry {
        Entry::Changed { staged, .. } | Entry::Renamed { staged, .. } => {
            *staged != Change::Unchanged
        }
        Entry::Conflicted { .. } => true,
        _ => false,
    }
}

/// Abandons the waiting merge under the repo lock: git puts back the state before it and keeps
/// the uncommitted changes the merge did not touch. The index syncs afterwards.
///
/// # Errors
///
/// [`StudioError::Lock`] when the lock cannot be taken, [`StudioError::NotInRepository`], and
/// [`StudioError::Git`] when git fails, also when no merge is in progress.
pub fn abort_merge(workspace: &mut Workspace, git: &Git) -> Result<(), StudioError> {
    {
        let held = workspace.hold("abort merge")?;
        let top = repo_top(git, held.project().root())?;
        merge::abort_merge(git, &top)?;
    }
    if let Err(error) = workspace.index_mut().sync() {
        tracing::warn!(%error, "the index could not sync after the merge was aborted");
    }
    Ok(())
}

/// The upstream of `branch` from its configuration: `(remote, branch)` from
/// `branch.<name>.remote` and `branch.<name>.merge` without `refs/heads/`, or `None` when either
/// is not set.
///
/// # Errors
///
/// The errors of [`repo::value`].
pub(crate) fn upstream(
    git: &Git,
    top: &Path,
    branch: &str,
) -> Result<Option<(String, String)>, GitError> {
    let Some(remote) = repo::value(git, top, &format!("branch.{branch}.remote"))? else {
        return Ok(None);
    };
    let Some(merge) = repo::value(git, top, &format!("branch.{branch}.merge"))? else {
        return Ok(None);
    };
    let name = merge
        .strip_prefix("refs/heads/")
        .unwrap_or(&merge)
        .to_owned();
    Ok(Some((remote, name)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_remote_upstream_is_its_tracking_ref() {
        assert_eq!(tracking_ref("origin", "main"), "refs/remotes/origin/main");
        assert_eq!(
            tracking_ref("my/remote", "a/b"),
            "refs/remotes/my/remote/a/b"
        );
    }

    #[test]
    fn a_local_upstream_is_the_branch_itself() {
        assert_eq!(tracking_ref(".", "main"), "refs/heads/main");
    }

    #[test]
    fn keys_fold_as_the_repository_does() {
        let folding = Folding {
            ignore_case: true,
            precompose: false,
        };
        assert_eq!(
            key(folding, b"Lore/T-900.md"),
            key(folding, b"lore/t-900.md")
        );
        assert_eq!(key(Folding::default(), b"\xff"), b"\xff".to_vec());
        assert_ne!(
            key(Folding::default(), b"Lore/T-900.md"),
            key(Folding::default(), b"lore/t-900.md")
        );
    }

    #[test]
    fn a_rename_counts_both_paths_and_is_staged() {
        let entry = Entry::Renamed {
            from: b"a.md".to_vec(),
            to: b"b.md".to_vec(),
            staged: Change::Renamed,
            unstaged: Change::Unchanged,
            score: 100,
        };
        let paths: Vec<&Vec<u8>> = entry_paths(&entry).collect();
        assert_eq!(paths, [&b"a.md".to_vec(), &b"b.md".to_vec()]);
        assert!(is_staged(&entry));
        assert!(!is_staged(&Entry::Untracked {
            path: b"c.md".to_vec()
        }));
    }
}
