//! A record's or a file's diff between two versions: staged, unstaged, a commit against its
//! parent, or any two versions.

use std::path::Path;

use theos_content::diff::{RecordDiff, WordDiff, record, words};
use theos_vcs::history::Commit;
use theos_vcs::read::{Content, Version, read};
use theos_vcs::{Folding, Git, GitError, repo};

use crate::StudioError;
use crate::changes::named::named;
use crate::repo_path::project_path;
use crate::top::repo_top;
use crate::workspace::Workspace;

/// What a diff compares.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Side<'a> {
    /// `HEAD` against the index.
    Staged,
    /// The index against the working tree.
    Unstaged,
    /// The commit's first parent against the commit, at the commit's own paths: the parent is
    /// read at [`Commit::from`] when the commit renamed the file, else at [`Commit::path`]. A
    /// commit without a parent compares with nothing.
    Commit(&'a Commit),
    /// Any two versions, the old one first, such as `main` against `agent/s-04`.
    Between(Version<'a>, Version<'a>),
}

/// The diff of one file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FileDiff {
    /// A record: its fields and its body.
    Record(RecordDiff),
    /// A text file that is not a record, word by word; a side that does not exist is empty.
    Text {
        /// The old text.
        old: String,
        /// The new text.
        new: String,
        /// The word diff of the two texts.
        words: WordDiff,
    },
    /// A side is not UTF-8 text, or is no regular file (a link, a submodule, a folder).
    Binary,
}

/// The diff of the file at the repository path `path` (git's spelling, relative to the top:
/// [`RecordChange::repo_path`](crate::RecordChange::repo_path), a status entry, a history
/// entry's path) between the two versions `side` names.
///
/// The new side is read at `path`. The old side is read at `from` (the path before a rename),
/// or at `path` when `from` is `None` or the old version holds nothing at `from` (a staged
/// rename seen from the index). [`Side::Commit`] reads at the commit's own paths instead of
/// `path` and `from`.
///
/// The file is a record when its new side, else its old side, is a record of a kind the
/// registry knows at a path in the project (as [`changes`](crate::changes()) decides); it is
/// then diffed with [`record()`], any other text with [`words()`]. Nothing is locked or written.
///
/// # Errors
///
/// [`StudioError::NotInRepository`] when the project folder is in no repository, and
/// [`StudioError::Git`] when git fails, a revision does not exist or a path is not UTF-8.
pub fn diff(
    workspace: &Workspace,
    git: &Git,
    path: &[u8],
    from: Option<&[u8]>,
    side: Side<'_>,
) -> Result<FileDiff, StudioError> {
    let root = workspace.project().root();
    let top = repo_top(git, root)?;
    let parent = match side {
        Side::Commit(commit) => repo::first_parent(git, &top, &commit.id)?,
        _ => None,
    };
    let (old_version, new_version, path, from) = match side {
        Side::Staged => (Some(Version::Head), Version::Index, path, from),
        Side::Unstaged => (Some(Version::Index), Version::WorkTree, path, from),
        Side::Commit(commit) => (
            parent.as_deref().map(Version::Rev),
            Version::Rev(&commit.id),
            commit.path.as_slice(),
            commit.from.as_deref(),
        ),
        Side::Between(old, new) => (Some(old), new, path, from),
    };
    let new = Loaded::from(read(git, &top, new_version, path)?);
    let (old, old_path) = match old_version {
        Some(version) => read_old(git, &top, version, path, from)?,
        None => (Loaded::Absent, path),
    };
    if matches!(old, Loaded::Binary) || matches!(new, Loaded::Binary) {
        return Ok(FileDiff::Binary);
    }
    let (old, new) = (old.text(), new.text());
    let is_record = {
        let place = repo::place(&top, root)?;
        let folding = Folding::read(git, &top)?;
        let names = |path: &[u8], text: Option<&str>| {
            text.zip(project_path(folding, &place, path))
                .is_some_and(|(text, project)| {
                    named(&project, text.as_bytes(), workspace.registry()).is_some()
                })
        };
        names(path, new) || names(old_path, old)
    };
    if is_record {
        return Ok(FileDiff::Record(record(old, new)));
    }
    let (old, new) = (old.unwrap_or_default(), new.unwrap_or_default());
    Ok(FileDiff::Text {
        words: words(old, new),
        old: old.to_owned(),
        new: new.to_owned(),
    })
}

/// The old side at `version`: at `from` when given and the version holds something there, else
/// at `path`; with the path it was read at.
///
/// # Errors
///
/// The errors of [`read`].
fn read_old<'p>(
    git: &Git,
    top: &Path,
    version: Version<'_>,
    path: &'p [u8],
    from: Option<&'p [u8]>,
) -> Result<(Loaded, &'p [u8]), GitError> {
    if let Some(from) = from {
        match read(git, top, version, from)? {
            Content::Absent => {}
            content => return Ok((Loaded::from(content), from)),
        }
    }
    Ok((Loaded::from(read(git, top, version, path)?), path))
}

/// One side of a diff as the diff sees it.
enum Loaded {
    /// The version holds nothing at the path.
    Absent,
    /// UTF-8 text.
    Text(String),
    /// Bytes that are not UTF-8, or no regular file.
    Binary,
}

impl From<Content> for Loaded {
    fn from(content: Content) -> Loaded {
        match content {
            Content::Absent => Loaded::Absent,
            Content::File(bytes) => String::from_utf8(bytes).map_or(Loaded::Binary, Loaded::Text),
            // `Content::NotAFile` (a link, a submodule, a folder). `Content` is non-exhaustive, so
            // the arm is a wildcard, which also takes a kind of content added later: no text either.
            _ => Loaded::Binary,
        }
    }
}

impl Loaded {
    /// The text, or `None` for a side that is absent or binary.
    fn text(&self) -> Option<&str> {
        match self {
            Loaded::Text(text) => Some(text),
            Loaded::Absent | Loaded::Binary => None,
        }
    }
}
