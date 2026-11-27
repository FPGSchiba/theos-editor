//! A record's history by id: the commits of its file, across renames of the file and the id.

use theos_content::RecordSummary;
use theos_vcs::history::{Commit, Page};
use theos_vcs::staged::{self, Tracked};
use theos_vcs::{Cancel, Folding, Git};

use crate::StudioError;
use crate::repo_path::project_path;
use crate::top::repo_top;
use crate::workspace::Workspace;

/// One commit of a record's history.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HistoryEntry {
    /// The commit, with the record's repository path in it ([`Commit::path`]): what
    /// [`diff`](crate::diff()) takes with [`Side::Commit`](crate::Side::Commit).
    pub commit: Commit,
    /// The record's project path in this commit, in git's spelling; it differs from today's after
    /// a rename. A path outside the project folder (a file moved into the project) is the
    /// repository path, relative to the top.
    pub path: String,
    /// The record's id in this commit: the file name of its path without `.md`.
    pub id: String,
}

/// The commits that changed the record `id`, newest first, following renames of its file and its
/// id; `page` picks which of them (see [`theos_vcs::history::history`]).
///
/// The record's path comes from the index ([`Index::record`](theos_content::Index::record)) and
/// is spelled as git's index spells it, compared under the repository's [`Folding`]. An id the
/// index does not know, and a record git's index does not hold (only in the working tree, or
/// added with intent to add), have an empty history; so does a record staged but never
/// committed. When several records share the id, the first one git tracks, in path order, is
/// followed. Nothing is locked or written.
///
/// # Errors
///
/// [`StudioError::NotInRepository`] when the project folder is in no repository,
/// [`StudioError::Index`] when the index fails, and [`StudioError::Git`] when git fails, or with
/// [`GitError::Cancelled`](theos_vcs::GitError::Cancelled) when `cancel` is set.
pub fn history(
    workspace: &Workspace,
    git: &Git,
    id: &str,
    page: Page,
    cancel: &Cancel,
) -> Result<Vec<HistoryEntry>, StudioError> {
    let records = workspace.index().record(id)?;
    if records.is_empty() {
        return Ok(Vec::new());
    }
    let root = workspace.project().root();
    repo_top(git, root)?;
    let staged = staged::read(git, root)?;
    let Some(repo_path) = git_spelling(&records, &staged.tracked, staged.folding, &staged.place)
    else {
        return Ok(Vec::new());
    };
    let commits = theos_vcs::history::history(git, &staged.top, repo_path, page, cancel)?;
    Ok(commits
        .into_iter()
        .map(|commit| entry(commit, staged.folding, &staged.place))
        .collect())
}

/// The repository path, in git's spelling, of the first of `records` that `tracked` holds: the
/// entry whose project path is the record's byte for byte, else the same under `folding`.
fn git_spelling<'a>(
    records: &[RecordSummary],
    tracked: &'a [Tracked],
    folding: Folding,
    place: &str,
) -> Option<&'a [u8]> {
    let in_project: Vec<(String, &[u8])> = tracked
        .iter()
        .filter_map(|entry| {
            Some((
                project_path(folding, place, &entry.path)?,
                entry.path.as_slice(),
            ))
        })
        .collect();
    records.iter().find_map(|record| {
        let exact = in_project.iter().find(|(path, _)| *path == record.path);
        let folded = || {
            let wanted = folding.fold(&record.path);
            in_project
                .iter()
                .find(|(path, _)| folding.fold(path) == wanted)
        };
        exact.or_else(folded).map(|(_, repo_path)| *repo_path)
    })
}

/// The history entry of `commit`: its path as a project path when it lies in the project at
/// `place`, else as the repository path, and the id from the file name.
fn entry(commit: Commit, folding: Folding, place: &str) -> HistoryEntry {
    let path = project_path(folding, place, &commit.path)
        .unwrap_or_else(|| String::from_utf8_lossy(&commit.path).into_owned());
    let name = path.rsplit('/').next().unwrap_or(&path);
    let id = name.strip_suffix(".md").unwrap_or(name).to_owned();
    HistoryEntry { commit, path, id }
}

#[cfg(test)]
mod tests {
    use theos_vcs::history::Person;
    use theos_vcs::status::Change;

    use super::*;

    const EXACT: Folding = Folding {
        ignore_case: false,
        precompose: false,
    };
    const CASE: Folding = Folding {
        ignore_case: true,
        precompose: false,
    };

    fn summary(path: &str) -> RecordSummary {
        let name = path.rsplit('/').next().unwrap_or(path);
        RecordSummary {
            id: name.trim_end_matches(".md").to_owned(),
            path: path.to_owned(),
            kind: Some("truth".to_owned()),
            title: None,
            status: None,
        }
    }

    fn tracked(path: &str) -> Tracked {
        Tracked {
            path: path.as_bytes().to_vec(),
            mode: 0o100_644,
            object: "0".repeat(40),
        }
    }

    fn commit(path: &[u8]) -> Commit {
        Commit {
            id: "0".repeat(40),
            author: Person {
                name: "Synthetic".to_owned(),
                email: "synthetic@example.invalid".to_owned(),
            },
            time: 0,
            offset_minutes: 0,
            subject: "synthetic".to_owned(),
            trailers: Vec::new(),
            change: Change::Modified,
            path: path.to_vec(),
            from: None,
        }
    }

    #[test]
    fn git_spells_the_path_of_the_first_tracked_record() {
        let records = [summary("lore/a/T-1.md"), summary("lore/b/T-1.md")];
        let index = [tracked("content/lore/b/T-1.md"), tracked("other/T-1.md")];
        assert_eq!(
            git_spelling(&records, &index, EXACT, "content"),
            Some(b"content/lore/b/T-1.md".as_slice())
        );
        assert_eq!(git_spelling(&records[..1], &index, EXACT, "content"), None);
        assert_eq!(git_spelling(&records, &index, EXACT, "other"), None);
    }

    #[test]
    fn git_spelling_folds_only_when_no_entry_matches_exactly() {
        let records = [summary("lore/t-1.md")];
        let index = [tracked("Content/lore/T-1.md")];
        assert_eq!(
            git_spelling(&records, &index, CASE, "content"),
            Some(b"Content/lore/T-1.md".as_slice())
        );
        assert_eq!(git_spelling(&records, &index, EXACT, "content"), None);
        let both = [tracked("lore/T-1.md"), tracked("lore/t-1.md")];
        assert_eq!(
            git_spelling(&records, &both, CASE, ""),
            Some(b"lore/t-1.md".as_slice())
        );
    }

    #[test]
    fn an_entry_names_the_record_by_its_file_name_in_the_commit() {
        let inside = entry(commit(b"content/lore/T-1.md"), EXACT, "content");
        assert_eq!(
            (inside.path.as_str(), inside.id.as_str()),
            ("lore/T-1.md", "T-1")
        );
        let outside = entry(commit(b"drafts/T-2.md"), EXACT, "content");
        assert_eq!(
            (outside.path.as_str(), outside.id.as_str()),
            ("drafts/T-2.md", "T-2")
        );
        let top = entry(commit(b"T-3"), EXACT, "");
        assert_eq!((top.path.as_str(), top.id.as_str()), ("T-3", "T-3"));
    }
}
