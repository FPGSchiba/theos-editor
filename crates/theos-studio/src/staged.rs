//! Judging git's staged snapshot of a project: what a commit would contain.

use std::collections::{BTreeMap, BTreeSet};

use theos_content::{Change, Finding};
use theos_vcs::staged::{self, regular_mode};
use theos_vcs::{Folding, Git};

use crate::StudioError;
use crate::check::counts;
use crate::top::repo_top;
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
/// counts as gone, because the commit does not contain it. git's index decides how a path is
/// spelled: a file the cache holds under another spelling (a case-only rename on a file system
/// that ignores case) is judged at git's spelling.
///
/// # Errors
///
/// [`StudioError::NotInRepository`] when the project folder is in no repository, other
/// [`StudioError::Git`] errors when git fails, and [`StudioError::Index`] when the index fails.
pub fn check_staged(workspace: &mut Workspace, git: &Git) -> Result<StagedCheck, StudioError> {
    let root = workspace.project().root().to_path_buf();
    repo_top(git, &root)?;
    let staged = staged::read(git, &root)?;
    let (place, folding) = (staged.place.as_str(), staged.folding);

    let changed: Vec<String> = staged
        .changed
        .iter()
        .filter_map(|entry| project_path(folding, place, &entry.path))
        .collect();
    let tracked: BTreeMap<String, Entry<'_>> = staged
        .tracked
        .iter()
        .filter_map(|entry| {
            Some((
                project_path(folding, place, &entry.path)?,
                Entry {
                    mode: entry.mode,
                    object: &entry.object,
                },
            ))
        })
        .collect();
    let modified: Vec<(String, Vec<u8>)> = staged
        .modified
        .iter()
        .filter_map(|(path, bytes)| Some((project_path(folding, place, path)?, bytes.clone())))
        .collect();

    let cached = workspace.index().paths()?;
    let edits = changes(&cached, &tracked, modified, folding, |objects| {
        staged::blobs(git, &staged.top, objects)
    })?;
    let findings = workspace.write(|index| index.snapshot(&edits))?;
    Ok(StagedCheck {
        findings: changed_first(findings, &changed),
        changed,
    })
}

/// An index entry of the project: its mode and its staged object.
#[derive(Clone, Copy, Debug)]
struct Entry<'a> {
    mode: u32,
    object: &'a str,
}

/// The path of `repo_path` below the project at `place`, compared by components under `folding`
/// and kept in git's spelling, or `None` when it lies outside the project or is not UTF-8.
fn project_path(folding: Folding, place: &str, repo_path: &[u8]) -> Option<String> {
    let rest = folding.below(place, repo_path)?;
    std::str::from_utf8(rest).ok().map(str::to_owned)
}

/// The changes that turn the working-tree index into the staged snapshot, in this order:
/// 1. the staged content of each `modified` path, in `modified` order;
/// 2. the staged content of each regular `tracked` path that the cache holds only under another
///    spelling (the same under `folding`) and that is not modified, in `tracked` order. Its
///    bytes come from `blobs`, asked once with the objects of all such paths;
/// 3. a removal for each `cached` path that is not a regular file in `tracked`, in `cached`
///    order. This removes the other spelling of a path of step 2.
///
/// # Errors
///
/// The error of `blobs`.
fn changes<E>(
    cached: &[String],
    tracked: &BTreeMap<String, Entry<'_>>,
    modified: Vec<(String, Vec<u8>)>,
    folding: Folding,
    blobs: impl FnOnce(&[&str]) -> Result<Vec<Vec<u8>>, E>,
) -> Result<Vec<Change>, E> {
    let held: BTreeSet<&str> = cached.iter().map(String::as_str).collect();
    let folded: BTreeSet<String> = cached
        .iter()
        .map(|path| folding.fold(path).into_owned())
        .collect();
    let edited: BTreeSet<&str> = modified.iter().map(|(path, _)| path.as_str()).collect();
    let respelled: Vec<(&str, &str)> = tracked
        .iter()
        .filter(|(path, entry)| {
            regular_mode(entry.mode)
                && !held.contains(path.as_str())
                && !edited.contains(path.as_str())
                && folded.contains(folding.fold(path).as_ref())
        })
        .map(|(path, entry)| (path.as_str(), entry.object))
        .collect();
    let objects: Vec<&str> = respelled.iter().map(|(_, object)| *object).collect();
    let bytes = blobs(&objects)?;

    let content = modified
        .into_iter()
        .chain(
            respelled
                .into_iter()
                .map(|(path, _)| path.to_owned())
                .zip(bytes),
        )
        .map(|(path, bytes)| Change::Content { path, bytes });
    let removed = cached
        .iter()
        .filter(|path| {
            !tracked
                .get(*path)
                .is_some_and(|entry| regular_mode(entry.mode))
        })
        .map(|path| Change::Removed { path: path.clone() });
    Ok(content.chain(removed).collect())
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
    use std::convert::Infallible;

    use super::*;

    const EXACT: Folding = Folding {
        ignore_case: false,
        precompose: false,
    };
    const CASE: Folding = Folding {
        ignore_case: true,
        precompose: false,
    };
    const NFC: Folding = Folding {
        ignore_case: false,
        precompose: true,
    };

    fn owned(paths: &[&str]) -> Vec<String> {
        paths.iter().map(|path| (*path).to_owned()).collect()
    }

    fn tracked<'a>(entries: &[(&str, u32, &'a str)]) -> BTreeMap<String, Entry<'a>> {
        entries
            .iter()
            .map(|(path, mode, object)| {
                (
                    (*path).to_owned(),
                    Entry {
                        mode: *mode,
                        object,
                    },
                )
            })
            .collect()
    }

    fn content(path: &str, bytes: &[u8]) -> Change {
        Change::Content {
            path: path.to_owned(),
            bytes: bytes.to_vec(),
        }
    }

    fn removed(path: &str) -> Change {
        Change::Removed {
            path: path.to_owned(),
        }
    }

    /// Answers each object with its own id as bytes, and records what was asked.
    fn blobs_of(
        asked: &mut Vec<String>,
    ) -> impl FnOnce(&[&str]) -> Result<Vec<Vec<u8>>, Infallible> + '_ {
        move |objects| {
            asked.extend(objects.iter().map(|object| (*object).to_owned()));
            Ok(objects
                .iter()
                .map(|object| object.as_bytes().to_vec())
                .collect())
        }
    }

    #[test]
    fn project_path_strips_the_place() {
        assert_eq!(
            project_path(EXACT, "", b"a/b.md").as_deref(),
            Some("a/b.md")
        );
        assert_eq!(
            project_path(EXACT, "content", b"content/a.md").as_deref(),
            Some("a.md")
        );
        assert_eq!(project_path(EXACT, "content", b"content2/x.md"), None);
        assert_eq!(project_path(EXACT, "content", b"other/a.md"), None);
        assert_eq!(project_path(EXACT, "", b"caf\xe9.md"), None);
    }

    #[test]
    fn project_path_compares_the_place_under_the_folding() {
        // The place differs in case: git's spelling of the rest is kept.
        assert_eq!(
            project_path(CASE, "Content", b"content/lore/A.md").as_deref(),
            Some("lore/A.md")
        );
        assert_eq!(project_path(EXACT, "Content", b"content/lore/A.md"), None);
        // The place is NFD on disk, NFC in the index.
        let decomposed = "Cafe\u{301}";
        let composed = "Caf\u{e9}/a.md";
        assert_eq!(
            project_path(NFC, decomposed, composed.as_bytes()).as_deref(),
            Some("a.md")
        );
        assert_eq!(project_path(EXACT, decomposed, composed.as_bytes()), None);
    }

    #[test]
    fn changes_cover_modified_untracked_and_irregular() {
        let cached = owned(&["a", "b", "c", "d", ".theos/settings.toml"]);
        let tracked = tracked(&[
            ("a", 0o100_644, "1"),
            ("b", 0o120_000, "2"),
            ("c", 0o100_644, "3"),
        ]);
        let modified = vec![("c".to_owned(), b"staged".to_vec())];
        let mut asked = Vec::new();
        assert_eq!(
            changes(&cached, &tracked, modified, CASE, blobs_of(&mut asked)),
            Ok(vec![
                content("c", b"staged"),
                removed("b"),
                removed("d"),
                removed(".theos/settings.toml"),
            ])
        );
        assert!(asked.is_empty(), "{asked:?}");
    }

    #[test]
    fn a_path_cached_under_another_spelling_gets_its_staged_content() {
        let cached = owned(&["lore/clues/c-900.md", "lore/z.md", "lore/x.md"]);
        let tracked = tracked(&[
            ("lore/X.md", 0o100_644, "1"),
            ("lore/clues/C-900.md", 0o100_644, "f00d"),
            ("lore/clues/C-901.md", 0o120_000, "beef"),
            ("lore/z.md", 0o100_644, "2"),
        ]);
        let modified = vec![("lore/z.md".to_owned(), b"staged".to_vec())];
        let mut asked = Vec::new();
        assert_eq!(
            changes(&cached, &tracked, modified, CASE, blobs_of(&mut asked)),
            Ok(vec![
                content("lore/z.md", b"staged"),
                content("lore/X.md", b"1"),
                content("lore/clues/C-900.md", b"f00d"),
                removed("lore/clues/c-900.md"),
                removed("lore/x.md"),
            ])
        );
        assert_eq!(asked, ["1", "f00d"]);
    }

    #[test]
    fn a_respelled_path_needs_the_folding_and_no_staged_bytes() {
        let cached = owned(&["lore/clues/c-900.md"]);
        let tracked = tracked(&[("lore/clues/C-900.md", 0o100_644, "f00d")]);
        // Byte for byte the two spellings are two paths: the cached one is not committed.
        let mut asked = Vec::new();
        assert_eq!(
            changes(&cached, &tracked, Vec::new(), EXACT, blobs_of(&mut asked)),
            Ok(vec![removed("lore/clues/c-900.md")])
        );
        assert!(asked.is_empty(), "{asked:?}");
        // A path git reports as modified already carries its staged bytes.
        let modified = vec![("lore/clues/C-900.md".to_owned(), b"staged".to_vec())];
        let mut asked = Vec::new();
        assert_eq!(
            changes(&cached, &tracked, modified, CASE, blobs_of(&mut asked)),
            Ok(vec![
                content("lore/clues/C-900.md", b"staged"),
                removed("lore/clues/c-900.md"),
            ])
        );
        assert!(asked.is_empty(), "{asked:?}");
    }

    #[test]
    fn a_failing_blob_read_fails_the_changes() {
        let cached = owned(&["c-900.md"]);
        let tracked = tracked(&[("C-900.md", 0o100_644, "f00d")]);
        let result = changes(&cached, &tracked, Vec::new(), CASE, |_| Err("no git"));
        assert_eq!(result, Err("no git"));
    }
}
