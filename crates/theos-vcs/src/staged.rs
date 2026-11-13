//! The staged snapshot: what a commit would contain, read through the git command line.

mod parse;

use std::collections::HashMap;
use std::ffi::OsStr;
use std::path::{Path, PathBuf};

use crate::folding::Folding;
use crate::git::{Git, GitError, args_text};
use crate::repo;

/// How git sees a staged path against `HEAD`.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Status {
    /// Added to the index.
    Added,
    /// Deleted from the index.
    Deleted,
    /// Its content or mode changed.
    Modified,
    /// The type changed (file, link).
    TypeChanged,
    /// A merge conflict.
    Unmerged,
    /// Any other status letter.
    Other(u8),
}

/// A path the index changes against `HEAD`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StagedPath {
    /// The repository-relative path, as git's bytes.
    pub path: Vec<u8>,
    /// What changes.
    pub status: Status,
}

/// A path in the index.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Tracked {
    /// The repository-relative path, as git's bytes.
    pub path: Vec<u8>,
    /// The file mode.
    pub mode: u32,
    /// The id of the staged object, as [`blobs`] reads it.
    pub object: String,
}

/// A repository-relative path and bytes that belong to it.
pub type PathBytes = (Vec<u8>, Vec<u8>);

/// The staged snapshot: what a commit would contain.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Staged {
    /// The top folder of the repository.
    pub top: PathBuf,
    /// The place of the requested folder in the repository, as git's index spells it: empty at
    /// the top, `a/b` below it.
    pub place: String,
    /// How the repository compares paths; `tracked` and `modified` are the entries that lie
    /// below `place` under it.
    pub folding: Folding,
    /// Every path the index changes against `HEAD`, in the whole repository.
    pub changed: Vec<StagedPath>,
    /// The index entries below the requested folder, at stage 0.
    pub tracked: Vec<Tracked>,
    /// `(path, staged bytes)` of regular tracked files whose work-tree copy differs or is gone.
    pub modified: Vec<PathBytes>,
}

impl Tracked {
    /// Whether the entry is a regular file (mode `100644` or `100755`), not a link or submodule.
    #[must_use]
    pub fn is_regular(&self) -> bool {
        regular_mode(self.mode)
    }
}

/// Whether a file mode is that of a regular file (`100644` or `100755`), not a link, a submodule
/// or a folder.
#[must_use]
pub fn regular_mode(mode: u32) -> bool {
    matches!(mode, 0o100_644 | 0o100_755)
}

/// Runs git in `top` and reads its answer with `parse`, naming the command when it is unusable.
fn query<T>(
    git: &Git,
    top: &Path,
    args: &[&str],
    stdin: Option<&[u8]>,
    parse: impl FnOnce(&[u8]) -> Result<T, String>,
) -> Result<T, GitError> {
    let args: Vec<&OsStr> = args.iter().map(OsStr::new).collect();
    let out = git.run(&args, top, stdin, &[])?;
    parse(&out).map_err(|message| GitError::Output {
        args: args_text(&args),
        message,
    })
}

/// Reads the staged snapshot of the repository that holds `folder`.
///
/// `tracked` and `modified` cover `folder` only; `changed` covers the whole repository. git's
/// index decides how a path is spelled: the index and the work-tree diff are read for the whole
/// repository, and the entries whose first components are `folder`'s place under the
/// repository's [`Folding`] are kept, in git's spelling. A path is in `modified` when it is a
/// regular tracked file whose work-tree copy differs from its staged version, or is gone; the
/// bytes are the staged ones. The runner's environment, such as `GIT_INDEX_FILE`, picks the
/// index.
///
/// # Errors
///
/// The errors of [`repo::top`], [`repo::place`] and [`Folding::read`], those of [`Git::run`],
/// and [`GitError::Output`] when an answer of git cannot be read.
pub fn read(git: &Git, folder: &Path) -> Result<Staged, GitError> {
    let top = repo::top(git, folder)?;
    let canonical = repo::place(&top, folder)?;
    let folding = Folding::read(git, &top)?;
    let changed = query(
        git,
        &top,
        &[
            "--no-optional-locks",
            "diff",
            "--cached",
            "--name-status",
            "--no-renames",
            "-z",
        ],
        None,
        parse::name_status,
    )?;
    let entries = query(
        git,
        &top,
        &["--no-optional-locks", "ls-files", "-s", "-z"],
        None,
        parse::ls_files,
    )?;
    let work_tree = query(
        git,
        &top,
        &[
            "--no-optional-locks",
            "diff",
            "--name-status",
            "--no-renames",
            "-z",
        ],
        None,
        parse::name_status,
    )?;
    let place = git_place(&entries, &canonical, folding);
    let tracked: Vec<Tracked> = entries
        .into_iter()
        .filter(|entry| folding.below(&canonical, &entry.path).is_some())
        .collect();
    let edited = work_tree.into_iter().map(|entry| entry.path).collect();
    let modified = staged_bytes(git, &top, &tracked, edited)?;
    Ok(Staged {
        top,
        place,
        folding,
        changed,
        tracked,
        modified,
    })
}

/// The place `canonical` as the first entry below it spells it, or `canonical` when no entry
/// lies below it.
fn git_place(entries: &[Tracked], canonical: &str, folding: Folding) -> String {
    if canonical.is_empty() {
        return String::new();
    }
    entries
        .iter()
        .find_map(|entry| {
            let rest = folding.below(canonical, &entry.path)?;
            // The place, without the `/` before the rest.
            let length = entry.path.len().checked_sub(rest.len() + 1)?;
            std::str::from_utf8(&entry.path[..length])
                .ok()
                .map(str::to_owned)
        })
        .unwrap_or_else(|| canonical.to_owned())
}

/// Reads the bytes of the git objects `objects` (object ids, such as [`Tracked::object`]) in one
/// `cat-file --batch` in `top`, in the order asked. Runs no git when `objects` is empty.
///
/// # Errors
///
/// The errors of [`Git::run`], and [`GitError::Output`] when an object is missing or the answer
/// cannot be read.
pub fn blobs(git: &Git, top: &Path, objects: &[&str]) -> Result<Vec<Vec<u8>>, GitError> {
    if objects.is_empty() {
        return Ok(Vec::new());
    }
    let mut input = Vec::new();
    for object in objects {
        input.extend_from_slice(object.as_bytes());
        input.push(b'\n');
    }
    query(
        git,
        top,
        &["--no-optional-locks", "cat-file", "--batch"],
        Some(&input),
        |out| parse::batch(out, objects.len()),
    )
}

/// Reads the staged bytes of the `edited` paths that are regular entries of `tracked`.
fn staged_bytes(
    git: &Git,
    top: &Path,
    tracked: &[Tracked],
    edited: Vec<Vec<u8>>,
) -> Result<Vec<PathBytes>, GitError> {
    let regular: HashMap<&[u8], &str> = tracked
        .iter()
        .filter(|entry| entry.is_regular())
        .map(|entry| (entry.path.as_slice(), entry.object.as_str()))
        .collect();
    let wanted: Vec<(Vec<u8>, &str)> = edited
        .into_iter()
        .filter_map(|path| regular.get(path.as_slice()).map(|object| (path, *object)))
        .collect();
    let objects: Vec<&str> = wanted.iter().map(|(_, object)| *object).collect();
    let bytes = blobs(git, top, &objects)?;
    Ok(wanted
        .into_iter()
        .map(|(path, _)| path)
        .zip(bytes)
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(path: &[u8]) -> Tracked {
        Tracked {
            path: path.to_vec(),
            mode: 0o100_644,
            object: "aaaa".to_owned(),
        }
    }

    #[test]
    fn the_place_is_spelled_as_the_index_spells_it() {
        let folding = Folding {
            ignore_case: true,
            precompose: false,
        };
        let entries = [entry(b"other/a.md"), entry(b"content/lore/a.md")];
        assert_eq!(git_place(&entries, "Content", folding), "content");
        assert_eq!(git_place(&entries, "", folding), "");
        assert_eq!(git_place(&entries, "missing", folding), "missing");
        assert_eq!(
            git_place(&entries, "Content", Folding::default()),
            "Content"
        );
    }
}
