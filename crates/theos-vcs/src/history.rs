//! A file's commits across renames of the file (`git log --follow`).

mod parse;

use std::path::Path;

use crate::Cancel;
use crate::git::{Git, GitError, args_text};
use crate::read_args::read_args;
use crate::status::Change;

/// One commit of a file's history.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Commit {
    /// The commit id, in hex.
    pub id: String,
    /// The author, name and email after `.mailmap`.
    pub author: Person,
    /// The author time, seconds since the epoch.
    pub time: i64,
    /// The author's time zone, in minutes east of UTC.
    pub offset_minutes: i32,
    /// The first line of the message.
    pub subject: String,
    /// The trailers (`Approved-by`, `Co-Authored-By`, …), unfolded, in order; other lines of the
    /// trailer block, such as `(cherry picked from commit …)`, are left out. Empty when a trailer
    /// value holds `0x1c` or `0x1d`, so the block cannot be split.
    pub trailers: Vec<Trailer>,
    /// How this commit changed the followed file. A merge shows its change against its first
    /// parent. A rename made on a branch that was merged with a merge commit shows on that branch
    /// as [`Change::Deleted`] of the old path, with no `from`: `--follow` already switched to the
    /// old path at the merge, which carries the [`Change::Renamed`].
    pub change: Change,
    /// The file's path in this commit (git's bytes, relative to the top, with `/`).
    pub path: Vec<u8>,
    /// The path before, when the commit renamed or copied the file.
    pub from: Option<Vec<u8>>,
}

/// A commit's author.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Person {
    /// The name.
    pub name: String,
    /// The email address.
    pub email: String,
}

/// A trailer of a commit message, such as `Approved-by: Jann`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Trailer {
    /// The key, as written (`Approved-by`).
    pub key: String,
    /// The value, with folded lines joined.
    pub value: String,
}

/// Which part of a history to read: `max` commits after the newest `skip`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Page {
    /// How many of the newest commits to leave out.
    pub skip: usize,
    /// How many commits to return at most.
    pub max: usize,
}

/// The format of one commit: a record separator (`0x1e`), then the id, the author's name and
/// email, the raw date (`<seconds> <±hhmm>`), the trailers and the subject, each ended by a NUL
/// (the subject by the one `-z` adds). No field can hold a NUL, so a name, an email or a subject
/// may hold any other byte. Trailers are separated by `0x1d`, a key from its value by `0x1c`;
/// `only` leaves out the lines of the trailer block that are not trailers.
const FORMAT: &str = "--format=%x1e%H%x00%aN%x00%aE%x00%ad%x00\
                      %(trailers:only,unfold,separator=%x1d,key_value_separator=%x1c)%x00%s";

/// The largest `--max-count` git reads (an `int`).
const GIT_INT_MAX: usize = 2_147_483_647;

/// The commits that changed the file at `path` (git's bytes: relative to `top`, with `/`,
/// UTF-8), newest first, following its renames with git's default threshold (50 %).
///
/// Every merge whose result for the file differs from its first parent is listed (`--follow`
/// does not drop a merge that took one side unchanged): a resolved conflict, and also a change
/// merged from a branch, which then lists twice, as the branch commit and as the merge. A listed
/// merge shows its change against its first parent. A path never committed, and every
/// path before the first commit, has an empty history. Names, subjects and trailers that are not
/// UTF-8 are read lossily.
///
/// # Errors
///
/// The errors of [`Git::run_cancellable`], [`GitError::Cancelled`] when `cancel` is set, and
/// [`GitError::Output`] when the path is not UTF-8 or an answer of git cannot be read.
pub fn history(
    git: &Git,
    top: &Path,
    path: &[u8],
    page: Page,
    cancel: &Cancel,
) -> Result<Vec<Commit>, GitError> {
    let path = std::str::from_utf8(path).map_err(|_| GitError::Output {
        args: vec!["log".to_owned(), "--follow".to_owned()],
        message: format!(
            "the path `{}` is not UTF-8; theos reads only UTF-8 paths",
            String::from_utf8_lossy(path)
        ),
    })?;
    if !has_commit(git, top, cancel)? {
        return Ok(Vec::new());
    }
    // `--skip` drops commits before `--follow` reads their diffs, so a skipped rename is never
    // followed and the older commits vanish (git 2.56). `--max-count` counts shown commits, so
    // git is asked for `skip + max` and the first `skip` are dropped here.
    let max = format!(
        "--max-count={}",
        page.skip.saturating_add(page.max).min(GIT_INT_MAX)
    );
    let args = read_args(
        "log",
        &[
            "--follow",
            "--find-renames",
            "--diff-merges=first-parent",
            "--name-status",
            "-z",
            "--date=raw",
            FORMAT,
            &max,
            "--",
            path,
        ],
    );
    let out = git.run_cancellable(&args, top, None, &[], cancel)?;
    let commits = parse::commits(&out).map_err(|message| GitError::Output {
        args: args_text(&args),
        message,
    })?;
    Ok(commits.into_iter().skip(page.skip).collect())
}

/// Whether `HEAD` names a commit; `false` on a branch before its first commit, where git
/// answers exit code 1 with nothing on stderr (`--quiet`).
fn has_commit(git: &Git, top: &Path, cancel: &Cancel) -> Result<bool, GitError> {
    let args = read_args("rev-parse", &["--verify", "--quiet", "HEAD^{commit}"]);
    match git.run_cancellable(&args, top, None, &[], cancel) {
        Ok(_) => Ok(true),
        Err(GitError::Failed {
            code: Some(1),
            stderr,
            ..
        }) if stderr.is_empty() => Ok(false),
        Err(error) => Err(error),
    }
}
