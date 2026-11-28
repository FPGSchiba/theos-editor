//! Questions about a repository: its top folder, a folder's place in it, its boolean settings,
//! its hooks folder and a commit's first parent.

use std::path::{Path, PathBuf};

use crate::git::{Git, GitError, args_text, failed};
use crate::read_args::read_args;
use crate::{Cancel, CommitId};

/// Runs a `rev-parse` in `cwd` and reads the answer as one line of UTF-8 text.
fn rev_parse(git: &Git, cwd: &Path, extra: &[&str]) -> Result<String, GitError> {
    let args = read_args("rev-parse", extra);
    let out = git.run(&args, cwd, None, &[])?;
    let text = String::from_utf8(out).map_err(|_| GitError::Output {
        args: args_text(&args),
        message: "the answer is not UTF-8".to_owned(),
    })?;
    // git ends its answer with one newline.
    Ok(match text.strip_suffix('\n') {
        Some(line) => line.to_owned(),
        None => text,
    })
}

/// `path` with the platform's separators: git answers `C:/a/b` on Windows, and joining more to
/// it would mix `/` and `\`.
fn native(path: &Path) -> PathBuf {
    path.components().collect()
}

/// The top folder of the work tree that holds `folder`, with the platform's separators.
///
/// # Errors
///
/// [`GitError::Failed`] when `folder` is in no repository, [`GitError::Output`] when git's
/// answer is not UTF-8, and the errors of [`Git::run`].
pub fn top(git: &Git, folder: &Path) -> Result<PathBuf, GitError> {
    rev_parse(git, folder, &["--show-toplevel"]).map(|path| native(Path::new(&path)))
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

/// Whether the boolean setting `key` (such as `core.ignorecase`) is true for the repository at
/// `top`. A setting that is not set is false.
///
/// # Errors
///
/// [`GitError::Failed`] when the value is not a boolean, and the errors of [`Git::run`].
pub fn flag(git: &Git, top: &Path, key: &str) -> Result<bool, GitError> {
    let args = read_args("config", &["--bool", "--get", key]);
    match git.run(&args, top, None, &[]) {
        // With `--bool` git answers `true` or `false`.
        Ok(out) => Ok(out.trim_ascii() == b"true"),
        // `--get` exits with 1 when the setting is not set.
        Err(GitError::Failed { code: Some(1), .. }) => Ok(false),
        Err(other) => Err(other),
    }
}

/// The value of the setting `key` for the repository at `top`, or `None` when it is not set.
///
/// # Errors
///
/// The errors of [`Git::run`], except the exit code 1 of an unset key.
pub fn value(git: &Git, top: &Path, key: &str) -> Result<Option<String>, GitError> {
    let args = read_args("config", &["--get", key]);
    match git.run(&args, top, None, &[]) {
        Ok(out) => Ok(Some(
            String::from_utf8_lossy(&out)
                .trim_end_matches(['\r', '\n'])
                .to_owned(),
        )),
        // `--get` exits with 1 when the setting is not set.
        Err(GitError::Failed { code: Some(1), .. }) => Ok(None),
        Err(other) => Err(other),
    }
}

/// The folder git runs hooks from for the work tree at `top`, honouring `core.hooksPath`, with
/// the platform's separators.
///
/// # Errors
///
/// The errors of [`top`].
pub fn hooks_dir(git: &Git, top: &Path) -> Result<PathBuf, GitError> {
    let path = rev_parse(git, top, &["--git-path", "hooks"])?;
    Ok(native(&top.join(path)))
}

/// The git folder shared by the main work tree and its linked work trees, with the platform's
/// separators.
///
/// # Errors
///
/// The errors of [`top`].
pub fn common_dir(git: &Git, top: &Path) -> Result<PathBuf, GitError> {
    rev_parse(git, top, &["--path-format=absolute", "--git-common-dir"])
        .map(|path| native(Path::new(&path)))
}

/// The id of the first parent of `commit` (any revision git accepts), or `None` for a root
/// commit.
///
/// # Errors
///
/// [`GitError::Failed`] when `commit` names no commit, [`GitError::Output`] when the answer is not
/// UTF-8, and the errors of [`Git::run`].
pub fn first_parent(git: &Git, top: &Path, commit: &str) -> Result<Option<String>, GitError> {
    let args = read_args(
        "rev-list",
        &[
            "--parents",
            "--max-count=1",
            "--end-of-options",
            commit,
            "--",
        ],
    );
    let out = git.run(&args, top, None, &[])?;
    let text = String::from_utf8(out).map_err(|_| GitError::Output {
        args: args_text(&args),
        message: "the answer is not UTF-8".to_owned(),
    })?;
    // One line: the commit's id, then the ids of its parents.
    Ok(text.split_ascii_whitespace().nth(1).map(str::to_owned))
}

/// The commit `HEAD` names, or `None` before the first commit.
///
/// # Errors
///
/// [`GitError::Failed`] when git fails for another reason, [`GitError::Output`] when the answer
/// is not UTF-8, and the errors of [`Git::output`].
pub fn head(git: &Git, top: &Path) -> Result<Option<CommitId>, GitError> {
    let args = read_args("rev-parse", &["-q", "--verify", "HEAD"]);
    let out = git.output(&args, top, None, &[], &Cancel::new())?;
    // `-q --verify` exits with 1 and says nothing when the name does not resolve.
    if out.code == Some(1) && out.stderr.is_empty() {
        return Ok(None);
    }
    if !out.success() {
        return Err(failed(&args, out));
    }
    let text = String::from_utf8(out.stdout).map_err(|_| GitError::Output {
        args: args_text(&args),
        message: "the answer is not UTF-8".to_owned(),
    })?;
    Ok(Some(CommitId(text.trim_end().to_owned())))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn native_paths_use_the_platform_separator() {
        // Paths compare by components, so the text is compared.
        let s = std::path::MAIN_SEPARATOR_STR;
        assert_eq!(
            native(Path::new("a/b/c")).as_os_str(),
            format!("a{s}b{s}c").as_str()
        );
        assert_eq!(
            native(&Path::new("top").join(".git/hooks")).as_os_str(),
            format!("top{s}.git{s}hooks").as_str()
        );
    }
}
