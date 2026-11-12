//! Questions about a repository: its top folder, a folder's place in it and its hooks folder.

use std::ffi::OsStr;
use std::path::{Path, PathBuf};

use crate::git::{Git, GitError};

/// Runs a `rev-parse` in `cwd` and reads the answer as one line of UTF-8 text.
fn rev_parse(git: &Git, cwd: &Path, extra: &[&str]) -> Result<String, GitError> {
    let mut args: Vec<&OsStr> = vec![OsStr::new("rev-parse")];
    args.extend(extra.iter().map(OsStr::new));
    let out = git.run(&args, cwd, None, &[])?;
    let text = String::from_utf8(out).map_err(|_| GitError::Output {
        args: owned(&args),
        message: "the answer is not UTF-8".to_owned(),
    })?;
    // git ends its answer with one newline.
    Ok(match text.strip_suffix('\n') {
        Some(line) => line.to_owned(),
        None => text,
    })
}

fn owned(args: &[&OsStr]) -> Vec<String> {
    args.iter()
        .map(|arg| arg.to_string_lossy().into_owned())
        .collect()
}

/// The top folder of the work tree that holds `folder`.
///
/// # Errors
///
/// [`GitError::Failed`] when `folder` is in no repository, [`GitError::Output`] when git's
/// answer is not UTF-8, and the errors of [`Git::run`].
pub fn top(git: &Git, folder: &Path) -> Result<PathBuf, GitError> {
    rev_parse(git, folder, &["--show-toplevel"]).map(PathBuf::from)
}

/// The place of `folder` in the repository at `top`: empty at the top, `a/b` below it.
///
/// Both paths are canonicalized first, and the answer always uses `/`.
///
/// # Errors
///
/// [`GitError::Io`] when a path cannot be canonicalized, and [`GitError::Output`] when `folder`
/// is not under `top` or its path is not UTF-8.
pub fn place(top: &Path, folder: &Path) -> Result<String, GitError> {
    let top = std::fs::canonicalize(top).map_err(|source| GitError::Io { source })?;
    let folder = std::fs::canonicalize(folder).map_err(|source| GitError::Io { source })?;
    let output = |message: &str| GitError::Output {
        args: vec![top.display().to_string(), folder.display().to_string()],
        message: message.to_owned(),
    };
    let below = folder
        .strip_prefix(&top)
        .map_err(|_| output("the folder is not under the top"))?;
    let parts = below
        .components()
        .map(|part| {
            part.as_os_str()
                .to_str()
                .ok_or_else(|| output("the path is not UTF-8"))
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(parts.join("/"))
}

/// The folder git runs hooks from for the work tree at `top`, honouring `core.hooksPath`.
///
/// # Errors
///
/// The errors of [`top`].
pub fn hooks_dir(git: &Git, top: &Path) -> Result<PathBuf, GitError> {
    let path = rev_parse(git, top, &["--git-path", "hooks"])?;
    Ok(top.join(path))
}

/// The git folder shared by the main work tree and its linked work trees.
///
/// # Errors
///
/// The errors of [`top`].
pub fn common_dir(git: &Git, top: &Path) -> Result<PathBuf, GitError> {
    rev_parse(git, top, &["--path-format=absolute", "--git-common-dir"]).map(PathBuf::from)
}
