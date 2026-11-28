//! Stage and unstage by record.

use theos_vcs::status::Entry;
use theos_vcs::{Git, stage};

use crate::StudioError;
use crate::changes::RecordChange;
use crate::top::repo_top;
use crate::workspace::Workspace;

/// Stages `records` and `files`, the entries of [`Changes`](crate::Changes), under the repo lock.
/// A renamed record stages both paths, so the index holds one rename.
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
    write(workspace, git, records, files, "stage", stage::stage)
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
    write(workspace, git, records, files, "unstage", stage::unstage)
}

/// The shape of `theos_vcs::stage::stage` and `unstage`.
type Write = fn(&Git, &std::path::Path, &[&[u8]]) -> Result<(), theos_vcs::GitError>;

fn write(
    workspace: &mut Workspace,
    git: &Git,
    records: &[&RecordChange],
    files: &[&Entry],
    purpose: &str,
    write: Write,
) -> Result<(), StudioError> {
    let paths = paths_of(records, files)?;
    let held = workspace.hold(purpose)?;
    let top = repo_top(git, held.project().root())?;
    let paths: Vec<&[u8]> = paths.iter().map(Vec::as_slice).collect();
    write(git, &top, &paths)?;
    Ok(())
}

/// The repository paths to hand to git, in git's spelling; a conflict is refused first.
fn paths_of(records: &[&RecordChange], files: &[&Entry]) -> Result<Vec<Vec<u8>>, StudioError> {
    let mut paths = Vec::new();
    for record in records {
        if record.conflict.is_some() {
            return Err(StudioError::Conflicted {
                id: record.id.clone(),
            });
        }
        paths.extend(record.repo_from.iter().cloned());
        paths.push(record.repo_path.clone());
    }
    for entry in files {
        match entry {
            Entry::Changed { path, .. } | Entry::Untracked { path } => paths.push(path.clone()),
            Entry::Renamed { from, to, .. } => {
                paths.push(from.clone());
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

    // AC: CE-22.1
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
