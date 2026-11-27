//! A file's bytes at a version: `HEAD`, the index, any revision, or the working tree.

use std::path::Path;

use crate::git::{Git, GitError, args_text};
use crate::read_args::read_args;
use crate::staged::{self, blobs, parse, regular_mode};

/// Where a file's bytes come from.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Version<'a> {
    /// The commit `HEAD` names.
    Head,
    /// The index, stage 0.
    Index,
    /// Any revision git accepts: a commit id, a branch such as `agent/s-04`.
    Rev(&'a str),
    /// The file on disk.
    WorkTree,
}

/// What a version holds at a path.
#[non_exhaustive]
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Content {
    /// The version holds nothing at the path (or there is no commit yet).
    Absent,
    /// A regular file and its bytes.
    File(Vec<u8>),
    /// A link, a submodule or a folder.
    NotAFile,
}

/// Reads the file at `path` (git's bytes: relative to `top`, with `/`, UTF-8) at `version`.
///
/// A path with `*`, `?`, `[`, `:` or a leading `-` is a path, never a pattern or an option. An
/// intent-to-add entry (`git add -N`) reads as [`Content::Absent`] from the index, as a commit
/// does not record it. A revision that does not exist reads every path as `Absent`, as a missing
/// `HEAD` does before the first commit. The bytes are the stored ones: no line-ending change.
///
/// # Errors
///
/// The errors of [`Git::run`], [`GitError::Output`] when an answer of git cannot be read or the
/// path is not UTF-8, and [`GitError::Io`] when the working-tree file cannot be read.
pub fn read(git: &Git, top: &Path, version: Version<'_>, path: &[u8]) -> Result<Content, GitError> {
    let path = std::str::from_utf8(path).map_err(|_| GitError::Output {
        args: Vec::new(),
        message: "the path is not UTF-8".to_owned(),
    })?;
    match version {
        Version::WorkTree => work_tree(top, path),
        Version::Index => index(git, top, path),
        Version::Head => commit(git, top, "HEAD", path),
        Version::Rev(rev) => commit(git, top, rev, path),
    }
}

fn work_tree(top: &Path, path: &str) -> Result<Content, GitError> {
    let file = top.join(path);
    let io = |source| GitError::Io { source };
    match std::fs::symlink_metadata(&file) {
        Ok(meta) if meta.is_file() => std::fs::read(&file).map(Content::File).map_err(io),
        Ok(_) => Ok(Content::NotAFile),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(Content::Absent),
        Err(error) => Err(io(error)),
    }
}

/// The object `spec` (`<rev>:<path>`, `:0:<path>`) names, or `None` when git says there is none:
/// exit code 1 with nothing on stderr, as `--quiet` makes it.
fn object(git: &Git, top: &Path, spec: &str) -> Result<Option<String>, GitError> {
    let args = read_args(
        "rev-parse",
        &["--verify", "--quiet", "--end-of-options", spec],
    );
    match git.run(&args, top, None, &[]) {
        Ok(out) => String::from_utf8(out)
            .map(|text| Some(text.trim().to_owned()))
            .map_err(|_| GitError::Output {
                args: args_text(&args),
                message: "the object id is not UTF-8".to_owned(),
            }),
        Err(GitError::Failed {
            code: Some(1),
            stderr,
            ..
        }) if stderr.is_empty() => Ok(None),
        Err(error) => Err(error),
    }
}

/// The bytes of `object` when `mode` is a regular file, else [`Content::NotAFile`].
fn bytes(git: &Git, top: &Path, mode: u32, object: &str) -> Result<Content, GitError> {
    if !regular_mode(mode) {
        return Ok(Content::NotAFile);
    }
    let mut blob = blobs(git, top, &[object])?;
    blob.pop()
        .map(Content::File)
        .ok_or_else(|| GitError::Output {
            args: vec!["cat-file".to_owned(), "--batch".to_owned()],
            message: "no object came back".to_owned(),
        })
}

/// A file of a commit: `ls-tree` gives the mode, `cat-file` the bytes.
fn commit(git: &Git, top: &Path, rev: &str, path: &str) -> Result<Content, GitError> {
    let Some(object) = object(git, top, &format!("{rev}:{path}"))? else {
        return Ok(Content::Absent);
    };
    let mode = staged::query(
        git,
        top,
        "ls-tree",
        &["-z", "--end-of-options", rev, "--", path],
        None,
        |out| {
            let text = std::str::from_utf8(out).map_err(|_| "the answer is not UTF-8")?;
            let (mode, _) = text.split_once(' ').ok_or("the answer has no mode")?;
            u32::from_str_radix(mode, 8).map_err(|_| format!("`{mode}` is not an octal mode"))
        },
    )?;
    bytes(git, top, mode, &object)
}

/// A file of the index at stage 0: `ls-files -s` gives the mode, `cat-file` the bytes.
fn index(git: &Git, top: &Path, path: &str) -> Result<Content, GitError> {
    let Some(object) = object(git, top, &format!(":0:{path}"))? else {
        return Ok(Content::Absent);
    };
    if staged::is_intent_to_add(git, top, path)? {
        return Ok(Content::Absent);
    }
    let entries = staged::query(
        git,
        top,
        "ls-files",
        &["-s", "-z", "--", path],
        None,
        parse::ls_files,
    )?;
    let entry = entries
        .iter()
        .find(|entry| entry.path == path.as_bytes())
        .ok_or_else(|| GitError::Output {
            args: vec!["ls-files".to_owned()],
            message: format!("the index has no entry for {path}"),
        })?;
    bytes(git, top, entry.mode, &object)
}
