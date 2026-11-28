//! Stage and unstage by record.

use theos_vcs::status::{Change, Entry};
use theos_vcs::{Git, stage};

use crate::StudioError;
use crate::changes::RecordChange;
use crate::top::repo_top;
use crate::workspace::Workspace;

/// Stages `records` and `files`, the entries of [`Changes`](crate::Changes), under the repo lock.
/// A renamed record whose rename is not staged yet stages both paths, so the index holds one
/// rename. What has nothing unstaged is left alone: git's `add` refuses a path that only `HEAD`
/// still has.
///
/// # Errors
///
/// [`StudioError::Conflicted`] when a record or file is in a merge conflict (nothing is staged),
/// [`StudioError::Lock`] when the repo lock cannot be taken, [`StudioError::NotInRepository`]
/// when the project is in no repository, and [`StudioError::Git`] when git fails.
pub fn stage_records(
    workspace: &mut Workspace,
    git: &Git,
    records: &[&RecordChange],
    files: &[&Entry],
) -> Result<(), StudioError> {
    write(
        workspace,
        git,
        records,
        files,
        Direction::Stage,
        stage::stage,
    )
}

/// Unstages `records` and `files`; the mirror of [`stage_records`], with the same errors.
///
/// # Errors
///
/// As [`stage_records`].
pub fn unstage_records(
    workspace: &mut Workspace,
    git: &Git,
    records: &[&RecordChange],
    files: &[&Entry],
) -> Result<(), StudioError> {
    write(
        workspace,
        git,
        records,
        files,
        Direction::Unstage,
        stage::unstage,
    )
}

/// The shape of `theos_vcs::stage::stage` and `unstage`.
type Write = fn(&Git, &std::path::Path, &[&[u8]]) -> Result<(), theos_vcs::GitError>;

/// Which way the paths move.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Direction {
    Stage,
    Unstage,
}

impl Direction {
    fn purpose(self) -> &'static str {
        match self {
            Direction::Stage => "stage",
            Direction::Unstage => "unstage",
        }
    }
}

fn write(
    workspace: &mut Workspace,
    git: &Git,
    records: &[&RecordChange],
    files: &[&Entry],
    direction: Direction,
    write: Write,
) -> Result<(), StudioError> {
    let paths = paths_of(records, files, direction)?;
    let held = workspace.hold(direction.purpose())?;
    let top = repo_top(git, held.project().root())?;
    let paths: Vec<&[u8]> = paths.iter().map(Vec::as_slice).collect();
    write(git, &top, &paths)?;
    Ok(())
}

/// Whether `add` has something to do for a path with this unstaged change.
fn has_unstaged(unstaged: Change, untracked: bool) -> bool {
    untracked || unstaged != Change::Unchanged
}

/// The repository paths to hand to git, in git's spelling; a conflict is refused first. Staging
/// leaves out what has nothing unstaged and the old side of a staged rename, which `git add`
/// refuses as a path that matches nothing; `reset` and `rm --cached` tolerate them.
fn paths_of(
    records: &[&RecordChange],
    files: &[&Entry],
    direction: Direction,
) -> Result<Vec<Vec<u8>>, StudioError> {
    let stage = direction == Direction::Stage;
    let mut paths = Vec::new();
    for record in records {
        if record.conflict.is_some() {
            return Err(StudioError::Conflicted {
                id: record.id.clone(),
            });
        }
        if stage && !has_unstaged(record.unstaged, record.untracked) {
            continue;
        }
        if !stage || record.unstaged == Change::Renamed {
            paths.extend(record.repo_from.iter().cloned());
        }
        paths.push(record.repo_path.clone());
    }
    for entry in files {
        match entry {
            Entry::Untracked { path } => paths.push(path.clone()),
            Entry::Changed { path, unstaged, .. } => {
                if !stage || has_unstaged(*unstaged, false) {
                    paths.push(path.clone());
                }
            }
            Entry::Renamed {
                from, to, unstaged, ..
            } => {
                if stage && !has_unstaged(*unstaged, false) {
                    continue;
                }
                if !stage || *unstaged == Change::Renamed {
                    paths.push(from.clone());
                }
                paths.push(to.clone());
            }
            Entry::Conflicted { path, .. } => {
                return Err(StudioError::Conflicted {
                    id: String::from_utf8_lossy(path).into_owned(),
                });
            }
            other => {
                return Err(StudioError::UnsupportedEntry {
                    entry: format!("{other:?}"),
                });
            }
        }
    }
    Ok(paths)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lock::LockError;
    use crate::test_support::{Fixture, TestResult};

    #[test]
    fn a_workspace_inside_hold_gets_reentrant_at_once() -> TestResult {
        let mut fixture = Fixture::new(&[])?;
        let git = Git::find()?;
        let mut held = fixture.workspace_mut().hold("synthetic")?;
        for write in [stage_records, unstage_records] {
            let result = write(&mut held, &git, &[], &[]);
            assert!(
                matches!(&result, Err(StudioError::Lock(LockError::Reentrant))),
                "{result:?}"
            );
        }
        Ok(())
    }
}
