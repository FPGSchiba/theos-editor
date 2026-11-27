//! git's status of a project grouped by record: each changed record named by its id, kind and
//! title, renames as one entry, and the changed files that are no records apart.

pub(crate) mod named;
mod versions;

use std::path::Path;

use theos_content::{KindRegistry, is_record_path};
use theos_vcs::status::{self, Branch, Change, Conflict, Entry};
use theos_vcs::{Cancel, Folding, Git, GitError, repo};

use crate::StudioError;
use crate::repo_path::project_path;
use crate::top::repo_top;
use crate::workspace::Workspace;
use named::{Named, named};
use versions::{first_of_conflict, newest};

/// The changes of a project, as git's status shows them.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Changes {
    /// The branch and its upstream.
    pub branch: Branch,
    /// One entry per record, in the order of the project paths.
    pub records: Vec<RecordChange>,
    /// Changed files that are not records: project settings, templates, files outside the
    /// project folder (repository paths), in the order of git's status.
    pub files: Vec<Entry>,
}

/// A changed record.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RecordChange {
    /// The id: the file name without `.md`.
    pub id: String,
    /// The kind, as the record's newest version names it.
    pub kind: Option<String>,
    /// The title, as the record's newest version has it.
    pub title: Option<String>,
    /// The project path now.
    pub path: String,
    /// The project path before, when the record was renamed from a path in the project.
    pub from: Option<String>,
    /// [`RecordChange::path`] in git's spelling, relative to the top: what a diff takes.
    pub repo_path: Vec<u8>,
    /// The path before a rename in git's spelling, relative to the top.
    pub repo_from: Option<Vec<u8>>,
    /// The change from `HEAD` to the index.
    pub staged: Change,
    /// The change from the index to the working tree.
    pub unstaged: Change,
    /// Whether git does not track the file yet; both changes are then
    /// [`Change::Unchanged`].
    pub untracked: bool,
    /// The merge conflict on the path; both changes are then [`Change::Unchanged`].
    pub conflict: Option<Conflict>,
}

/// Lists the changes of the project of `workspace`: git's status, with each path in the project
/// folder whose newest version (the working tree; for a path gone from it, the index, else
/// `HEAD`) is a record of a known kind as a [`RecordChange`], and every other path in
/// [`Changes::files`]. A conflicted path is named by the first of its working copy and `HEAD` that
/// is a record, since conflict markers in the front matter break the working copy.
///
/// Which paths are records is decided from their bytes, not from the index: nothing is synced and
/// the cache is not written. No lock is taken.
///
/// # Errors
///
/// [`StudioError::NotInRepository`] when the project folder is in no repository,
/// [`StudioError::Git`] when git fails, or with [`GitError::Cancelled`] when `cancel` is set
/// while the status or the records are read.
pub fn changes(workspace: &Workspace, git: &Git, cancel: &Cancel) -> Result<Changes, StudioError> {
    let root = workspace.project().root();
    let top = repo_top(git, root)?;
    let project = InRepo {
        git,
        place: repo::place(&top, root)?,
        folding: Folding::read(git, &top)?,
        registry: workspace.registry(),
        top: &top,
    };
    let status = status::status(git, &top, cancel)?;
    let (records, files) = group(status.entries, cancel, |entry| project.record(entry))?;
    Ok(Changes {
        branch: status.branch,
        records,
        files,
    })
}

/// The arguments a cancel between the reads of the records names: the reads take their bytes with
/// `cat-file --batch`.
const READ_ARGS: [&str; 2] = ["cat-file", "--batch"];

/// Splits `entries` into record changes, sorted by project path, and the other entries, in their
/// order, with `record`. `cancel` is checked before each entry.
///
/// # Errors
///
/// [`GitError::Cancelled`] when `cancel` is set, and the errors of `record`.
fn group(
    entries: Vec<Entry>,
    cancel: &Cancel,
    mut record: impl FnMut(&Entry) -> Result<Option<RecordChange>, GitError>,
) -> Result<(Vec<RecordChange>, Vec<Entry>), GitError> {
    let mut records = Vec::new();
    let mut files = Vec::new();
    for entry in entries {
        if cancel.is_cancelled() {
            return Err(GitError::Cancelled {
                args: READ_ARGS.map(str::to_owned).to_vec(),
            });
        }
        match record(&entry)? {
            Some(record) => records.push(record),
            None => files.push(entry),
        }
    }
    records.sort_by(|a, b| a.path.cmp(&b.path));
    Ok((records, files))
}

/// The project in its repository: what turns a status entry into a record change.
struct InRepo<'a> {
    git: &'a Git,
    top: &'a Path,
    /// The project folder below the top, with `/`.
    place: String,
    folding: Folding,
    registry: &'a KindRegistry,
}

impl InRepo<'_> {
    /// The record change of `entry`, or `None` when its path is outside the project or is no
    /// record.
    fn record(&self, entry: &Entry) -> Result<Option<RecordChange>, GitError> {
        let unchanged = (Change::Unchanged, Change::Unchanged);
        let (repo_path, repo_from, (staged, unstaged), untracked, conflict) = match entry {
            Entry::Changed {
                path,
                staged,
                unstaged,
            } => (path, None, (*staged, *unstaged), false, None),
            Entry::Renamed {
                from,
                to,
                staged,
                unstaged,
                ..
            } => (to, Some(from), (*staged, *unstaged), false, None),
            Entry::Conflicted { path, conflict } => (path, None, unchanged, false, Some(*conflict)),
            Entry::Untracked { path } => (path, None, unchanged, true, None),
            _ => return Ok(None),
        };
        let Some(path) = project_path(self.folding, &self.place, repo_path) else {
            return Ok(None);
        };
        let Some(name) = self.name(&path, repo_path, conflict.is_some())? else {
            return Ok(None);
        };
        Ok(Some(RecordChange {
            id: name.id,
            kind: Some(name.kind),
            title: name.title,
            from: repo_from.and_then(|from| project_path(self.folding, &self.place, from)),
            path,
            repo_path: repo_path.clone(),
            repo_from: repo_from.cloned(),
            staged,
            unstaged,
            untracked,
            conflict,
        }))
    }

    /// The name of the record at the project path `path` (`repo_path` in git's spelling): from
    /// its newest version, or for a `conflicted` path from the first of the working copy and
    /// `HEAD` that is a record. A path that is no record path ([`is_record_path`]) is not read.
    fn name(
        &self,
        path: &str,
        repo_path: &[u8],
        conflicted: bool,
    ) -> Result<Option<Named>, GitError> {
        if !is_record_path(path) {
            return Ok(None);
        }
        let of = |bytes: &[u8]| named(path, bytes, self.registry);
        if conflicted {
            return first_of_conflict(self.git, self.top, repo_path, of);
        }
        Ok(newest(self.git, self.top, repo_path)?.and_then(|bytes| of(&bytes)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::TestResult;

    fn untracked(path: &str) -> Entry {
        Entry::Untracked {
            path: path.as_bytes().to_vec(),
        }
    }

    #[test]
    fn a_cancel_between_the_entries_stops_the_grouping() {
        let cancel = Cancel::new();
        let mut seen = Vec::new();
        let result = group(
            vec![untracked("a.md"), untracked("b.md"), untracked("c.md")],
            &cancel,
            |entry| {
                seen.push(entry.clone());
                cancel.cancel();
                Ok(None)
            },
        );
        assert!(
            matches!(&result, Err(GitError::Cancelled { args }) if args == &READ_ARGS),
            "{result:?}"
        );
        assert_eq!(seen, [untracked("a.md")]);
    }

    #[test]
    fn without_a_cancel_every_entry_is_grouped() -> TestResult {
        let result = group(
            vec![untracked("b.md"), untracked("x.txt"), untracked("a.md")],
            &Cancel::new(),
            |entry| {
                let Entry::Untracked { path } = entry else {
                    return Ok(None);
                };
                let path = String::from_utf8_lossy(path).into_owned();
                let Some(id) = path.strip_suffix(".md").map(str::to_owned) else {
                    return Ok(None);
                };
                Ok(Some(RecordChange {
                    id,
                    kind: None,
                    title: None,
                    repo_path: path.clone().into_bytes(),
                    path,
                    from: None,
                    repo_from: None,
                    staged: Change::Unchanged,
                    unstaged: Change::Unchanged,
                    untracked: true,
                    conflict: None,
                }))
            },
        );
        let (records, files) = result?;
        let ids: Vec<&str> = records.iter().map(|record| record.id.as_str()).collect();
        assert_eq!(ids, ["a", "b"]);
        assert_eq!(files, [untracked("x.txt")]);
        Ok(())
    }
}
