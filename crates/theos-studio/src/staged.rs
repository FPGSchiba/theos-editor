//! Judging git's staged snapshot of a project: what a commit would contain.

use std::collections::{BTreeMap, BTreeSet};

use theos_content::{Change, Finding};
use theos_vcs::staged::{self, regular_mode};
use theos_vcs::{Git, GitError, repo};

use crate::StudioError;
use crate::check::counts;
use crate::workspace::Workspace;

/// The verdict on a staged snapshot.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StagedCheck {
    /// Every finding of the snapshot: those on `changed` paths first, then the rest, each group
    /// in the order of the index.
    pub findings: Vec<Finding>,
    /// The project-relative paths the commit changes.
    pub changed: Vec<String>,
}

impl StagedCheck {
    /// Whether any finding is an error: the commit must not go through.
    #[must_use]
    pub fn blocks(&self) -> bool {
        self.errors() > 0
    }

    /// The number of findings that are errors.
    #[must_use]
    pub fn errors(&self) -> usize {
        counts(&self.findings).0
    }

    /// The number of findings that are warnings.
    #[must_use]
    pub fn warnings(&self) -> usize {
        counts(&self.findings).1
    }
}

/// Judges what git would commit: the findings of the project with the staged versions of its
/// files in place of the working copies. The working tree, the cache on disk and the repository
/// are left as they are, and no lock is taken.
///
/// A path that is not a regular file in the index (untracked, excluded, a link or a submodule)
/// counts as gone, because the commit does not contain it.
///
/// # Errors
///
/// [`StudioError::NotInRepository`] when the project folder is in no repository, other
/// [`StudioError::Git`] errors when git fails, and [`StudioError::Index`] when the index fails.
pub fn check_staged(workspace: &mut Workspace, git: &Git) -> Result<StagedCheck, StudioError> {
    let root = workspace.project().root().to_path_buf();
    repo::top(git, &root).map_err(|error| match error {
        GitError::Failed { .. } => StudioError::NotInRepository {
            folder: root.clone(),
        },
        other => StudioError::Git(other),
    })?;
    let staged = staged::read(git, &root)?;
    let place = repo::place(&staged.top, &root)?;

    let changed: Vec<String> = staged
        .changed
        .iter()
        .filter_map(|entry| project_path(&place, &entry.path))
        .collect();
    let tracked: BTreeMap<String, u32> = staged
        .tracked
        .iter()
        .filter_map(|entry| Some((project_path(&place, &entry.path)?, entry.mode)))
        .collect();
    let modified: Vec<(String, Vec<u8>)> = staged
        .modified
        .into_iter()
        .filter_map(|(path, bytes)| Some((project_path(&place, &path)?, bytes)))
        .collect();

    let cached = workspace.index().paths()?;
    let edits = changes(&cached, &tracked, modified);
    let findings = workspace.write(|index| index.snapshot(&edits))?;
    Ok(StagedCheck {
        findings: changed_first(findings, &changed),
        changed,
    })
}

/// The path of `repo_path` below the project at `place`, or `None` when it lies outside the
/// project or is not UTF-8.
fn project_path(place: &str, repo_path: &[u8]) -> Option<String> {
    let path = std::str::from_utf8(repo_path).ok()?;
    if place.is_empty() {
        return Some(path.to_owned());
    }
    path.strip_prefix(place)?
        .strip_prefix('/')
        .map(str::to_owned)
}

/// The changes that turn the working-tree index into the staged snapshot: the staged content of
/// each `modified` path, then a removal for each `cached` path that is not a regular file in
/// `tracked`.
fn changes(
    cached: &[String],
    tracked: &BTreeMap<String, u32>,
    modified: Vec<(String, Vec<u8>)>,
) -> Vec<Change> {
    let content = modified
        .into_iter()
        .map(|(path, bytes)| Change::Content { path, bytes });
    let removed = cached
        .iter()
        .filter(|path| !tracked.get(*path).is_some_and(|mode| regular_mode(*mode)))
        .map(|path| Change::Removed { path: path.clone() });
    content.chain(removed).collect()
}

/// The findings on `changed` paths, then the others; each group keeps its order.
fn changed_first(findings: Vec<Finding>, changed: &[String]) -> Vec<Finding> {
    let changed: BTreeSet<&str> = changed.iter().map(String::as_str).collect();
    let (first, rest): (Vec<Finding>, Vec<Finding>) = findings
        .into_iter()
        .partition(|finding| changed.contains(finding.path.as_str()));
    first.into_iter().chain(rest).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn project_path_strips_the_place() {
        assert_eq!(project_path("", b"a/b.md").as_deref(), Some("a/b.md"));
        assert_eq!(
            project_path("content", b"content/a.md").as_deref(),
            Some("a.md")
        );
        assert_eq!(project_path("content", b"content2/x.md"), None);
        assert_eq!(project_path("content", b"other/a.md"), None);
        assert_eq!(project_path("", b"caf\xe9.md"), None);
    }

    #[test]
    fn changes_cover_modified_untracked_and_irregular() {
        let cached: Vec<String> = ["a", "b", "c", "d", ".theos/settings.toml"]
            .map(str::to_owned)
            .to_vec();
        let tracked = BTreeMap::from([
            ("a".to_owned(), 0o100_644),
            ("b".to_owned(), 0o120_000),
            ("c".to_owned(), 0o100_644),
        ]);
        let modified = vec![("c".to_owned(), b"staged".to_vec())];
        assert_eq!(
            changes(&cached, &tracked, modified),
            vec![
                Change::Content {
                    path: "c".to_owned(),
                    bytes: b"staged".to_vec()
                },
                Change::Removed {
                    path: "b".to_owned()
                },
                Change::Removed {
                    path: "d".to_owned()
                },
                Change::Removed {
                    path: ".theos/settings.toml".to_owned()
                },
            ]
        );
    }
}
