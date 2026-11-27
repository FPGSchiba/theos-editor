//! git's status of a project grouped by record: each changed record named by its id, kind and
//! title, renames as one entry, and the changed files that are no records apart.

mod named;
mod newest;

use std::path::Path;

use theos_content::KindRegistry;
use theos_vcs::status::{self, Branch, Change, Conflict, Entry};
use theos_vcs::{Cancel, Folding, Git, GitError, repo};

use crate::StudioError;
use crate::repo_path::project_path;
use crate::top::repo_top;
use crate::workspace::Workspace;
use named::named;
use newest::newest;

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
/// [`Changes::files`].
///
/// Which paths are records is decided from their bytes, not from the index: nothing is synced and
/// the cache is not written. No lock is taken.
///
/// # Errors
///
/// [`StudioError::NotInRepository`] when the project folder is in no repository,
/// [`StudioError::Git`] when git fails or `cancel` is set while the status is read.
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
    let mut records = Vec::new();
    let mut files = Vec::new();
    for entry in status.entries {
        match project.record(&entry)? {
            Some(record) => records.push(record),
            None => files.push(entry),
        }
    }
    records.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(Changes {
        branch: status.branch,
        records,
        files,
    })
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
        let Some(bytes) = newest(self.git, self.top, repo_path)? else {
            return Ok(None);
        };
        let Some(name) = named(&path, &bytes, self.registry) else {
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
}
