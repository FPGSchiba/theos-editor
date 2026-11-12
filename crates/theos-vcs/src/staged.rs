//! The staged snapshot: what a commit would contain, read through the git command line.

mod parse;

use std::collections::HashMap;
use std::ffi::OsStr;
use std::path::{Path, PathBuf};

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
    /// `(path, staged bytes)` of regular tracked files whose work-tree copy differs or is gone.
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
}

/// A repository-relative path and bytes that belong to it.
pub type PathBytes = (Vec<u8>, Vec<u8>);

/// The staged snapshot: what a commit would contain.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Staged {
    /// The top folder of the repository.
    pub top: PathBuf,
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
/// `tracked` and `modified` cover `folder` only; `changed` covers the whole repository. A path
/// is in `modified` when it is a regular tracked file whose work-tree copy differs from its
/// staged version, or is gone; the bytes are the staged ones. The runner's environment, such as
/// `GIT_INDEX_FILE`, picks the index.
///
/// # Errors
///
/// The errors of [`repo::top`] and [`repo::place`], those of [`Git::run`], and
/// [`GitError::Output`] when an answer of git cannot be read.
pub fn read(git: &Git, folder: &Path) -> Result<Staged, GitError> {
    let top = repo::top(git, folder)?;
    let place = repo::place(&top, folder)?;
    let pathspec = if place.is_empty() { "." } else { &place };
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
        &[
            "--no-optional-locks",
            "--literal-pathspecs",
            "ls-files",
            "-s",
            "-z",
            "--",
            pathspec,
        ],
        None,
        parse::ls_files,
    )?;
    let edited = query(
        git,
        &top,
        &[
            "--no-optional-locks",
            "--literal-pathspecs",
            "diff",
            "--name-only",
            "--no-renames",
            "-z",
            "--",
            pathspec,
        ],
        None,
        parse::name_only,
    )?;
    let modified = staged_bytes(git, &top, &entries, edited)?;
    let tracked = entries
        .into_iter()
        .map(|entry| Tracked {
            path: entry.path,
            mode: entry.mode,
        })
        .collect();
    Ok(Staged {
        top,
        changed,
        tracked,
        modified,
    })
}

/// Reads the staged bytes of the `edited` paths that are regular entries, in one `cat-file`.
fn staged_bytes(
    git: &Git,
    top: &Path,
    entries: &[parse::Entry],
    edited: Vec<Vec<u8>>,
) -> Result<Vec<PathBytes>, GitError> {
    let regular: HashMap<&[u8], &str> = entries
        .iter()
        .filter(|entry| regular_mode(entry.mode))
        .map(|entry| (entry.path.as_slice(), entry.object.as_str()))
        .collect();
    let wanted: Vec<(Vec<u8>, &str)> = edited
        .into_iter()
        .filter_map(|path| regular.get(path.as_slice()).map(|object| (path, *object)))
        .collect();
    if wanted.is_empty() {
        return Ok(Vec::new());
    }
    let mut input = Vec::new();
    for (_, object) in &wanted {
        input.extend_from_slice(object.as_bytes());
        input.push(b'\n');
    }
    let blobs = query(
        git,
        top,
        &["--no-optional-locks", "cat-file", "--batch"],
        Some(&input),
        |out| parse::batch(out, wanted.len()),
    )?;
    Ok(wanted
        .into_iter()
        .map(|(path, _)| path)
        .zip(blobs)
        .collect())
}
