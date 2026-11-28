//! A record's history by id: the commits of its file, across renames of the file and the id.

use theos_content::RecordSummary;
use theos_vcs::history::{Commit, Page};
use theos_vcs::status::{Change, Entry};
use theos_vcs::{Cancel, Folding, Git, GitError, index, repo};

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
/// is spelled as git's index spells it ([`index::paths`]), compared under the repository's
/// [`Folding`]. When git's index does not hold the path, or no commit holds it yet, a rename not
/// yet committed (staged or not, paired by [`theos_vcs::status::status`]) whose new path is the
/// record's is followed from its old path, so a record renamed with P06's `Rename` keeps its
/// history before the commit. An id the index does not know and a record only in the working
/// tree, or staged but never committed, without such a rename have an empty history. When several
/// records share the id, the first one git tracks, in path order, is followed. Nothing is locked
/// or written.
///
/// # Errors
///
/// [`StudioError::NotInRepository`] when the project folder is in no repository,
/// [`StudioError::Index`] when the index fails, and [`StudioError::Git`] when git fails, or with
/// [`GitError::Cancelled`] when `cancel` is set, also before the call.
pub fn history(
    workspace: &Workspace,
    git: &Git,
    id: &str,
    page: Page,
    cancel: &Cancel,
) -> Result<Vec<HistoryEntry>, StudioError> {
    if cancel.is_cancelled() {
        return Err(GitError::Cancelled {
            args: CANCELLED_ARGS.map(str::to_owned).to_vec(),
        }
        .into());
    }
    let records = workspace.index().record(id)?;
    if records.is_empty() {
        return Ok(Vec::new());
    }
    let root = workspace.project().root();
    let top = repo_top(git, root)?;
    let place = repo::place(&top, root)?;
    let folding = Folding::read(git, &top)?;
    let tracked = index::paths(git, &top, cancel)?;
    let mut commits = match git_spelling(&records, &tracked, folding, &place) {
        Some(repo_path) => theos_vcs::history::history(git, &top, repo_path, page, cancel)?,
        None => Vec::new(),
    };
    if commits.is_empty() {
        let status = theos_vcs::status::status(git, &top, cancel)?;
        let renames = renames(&status.entries);
        if let Some(from) = matching(&records, &renames, folding, &place) {
            commits = theos_vcs::history::history(git, &top, from, page, cancel)?;
        }
    }
    Ok(commits
        .into_iter()
        .map(|commit| entry(commit, folding, &place))
        .collect())
}

/// The arguments a cancel before the start names: the history's `git log --follow`.
const CANCELLED_ARGS: [&str; 2] = ["log", "--follow"];

/// The repository path, in git's spelling, of the first of `records` that `tracked` (the paths of
/// git's index) holds: the entry whose project path is the record's byte for byte, else the same
/// under `folding`.
fn git_spelling<'a>(
    records: &[RecordSummary],
    tracked: &'a [Vec<u8>],
    folding: Folding,
    place: &str,
) -> Option<&'a [u8]> {
    let pairs: Vec<(&[u8], &[u8])> = tracked
        .iter()
        .map(|path| (path.as_slice(), path.as_slice()))
        .collect();
    matching(records, &pairs, folding, place)
}

/// The renames of `entries` not yet committed, staged or in the working tree, as (new path, old
/// path) in git's spelling.
fn renames(entries: &[Entry]) -> Vec<(&[u8], &[u8])> {
    entries
        .iter()
        .filter_map(|entry| match entry {
            Entry::Renamed {
                from,
                to,
                staged,
                unstaged,
                ..
            } if *staged == Change::Renamed || *unstaged == Change::Renamed => {
                Some((to.as_slice(), from.as_slice()))
            }
            _ => None,
        })
        .collect()
}

/// For the first of `records` that a pair's key names, the pair's value: the pairs are
/// (repository path, path to answer) in git's spelling; a key matches when its project path is
/// the record's byte for byte, else the same under `folding`.
fn matching<'a>(
    records: &[RecordSummary],
    pairs: &[(&[u8], &'a [u8])],
    folding: Folding,
    place: &str,
) -> Option<&'a [u8]> {
    let in_project: Vec<(String, &[u8])> = pairs
        .iter()
        .filter_map(|(key, value)| Some((project_path(folding, place, key)?, *value)))
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

    fn tracked(path: &str) -> Vec<u8> {
        path.as_bytes().to_vec()
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

    #[test]
    fn renames_pair_the_new_path_with_the_old_one_staged_or_not() {
        let renamed = |from: &str, to: &str, staged, unstaged| Entry::Renamed {
            from: tracked(from),
            to: tracked(to),
            staged,
            unstaged,
            score: 90,
        };
        let entries = [
            renamed("a.md", "b.md", Change::Unchanged, Change::Renamed),
            renamed("c.md", "d.md", Change::Renamed, Change::Modified),
            renamed("e.md", "f.md", Change::Copied, Change::Unchanged),
            Entry::Untracked {
                path: tracked("g.md"),
            },
        ];
        assert_eq!(
            renames(&entries),
            [
                (b"b.md".as_slice(), b"a.md".as_slice()),
                (b"d.md".as_slice(), b"c.md".as_slice()),
            ]
        );
        let records = [summary("lore/D.md")];
        let pairs = [(
            b"content/lore/d.md".as_slice(),
            b"content/lore/c.md".as_slice(),
        )];
        assert_eq!(
            matching(&records, &pairs, CASE, "content"),
            Some(b"content/lore/c.md".as_slice())
        );
        assert_eq!(matching(&records, &pairs, EXACT, "content"), None);
    }
}
