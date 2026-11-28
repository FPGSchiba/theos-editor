//! `commit`: a message and trailers, written by git itself with its hooks running.
//!
//! Follows spec P47 §5.5.

use std::path::Path;

use crate::git::args_text;
use crate::write_args::{checked, write_args_with, write_with};
use crate::{Cancel, CommitId, Git, GitError, repo};

/// The characters tried, in order, as `core.commentChar` for one commit.
const COMMENT_CHARS: [char; 10] = ['#', ';', '@', '!', '$', '%', '^', '&', '|', ':'];

/// Settings that keep a user's `trailer.*` configuration from moving the trailers: they are added
/// last, and a repeated neighbour is not added twice.
const TRAILER_SETTINGS: [&str; 3] = [
    "trailer.where=end",
    "trailer.ifexists=addIfDifferentNeighbor",
    "trailer.ifmissing=add",
];

/// A trailer of the commit message, such as `Approved-by: Jann`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Trailer {
    /// The key: not empty, only ASCII letters, digits and `-`.
    pub key: String,
    /// The value: one line.
    pub value: String,
}

/// Why a commit was not made.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum CommitRefused {
    /// The message is empty after trimming, or uses every comment character at a line start.
    #[error("invalid commit message: {0}")]
    Message(String),
    /// A trailer has a bad key or a value of more than one line.
    #[error("invalid trailer: {0}")]
    Trailer(String),
    /// Git failed, a hook refusing the commit included.
    #[error(transparent)]
    Git(#[from] GitError),
}

/// The first of [`COMMENT_CHARS`] that starts no line of `message`, `None` when each one does.
pub(crate) fn comment_char(message: &str) -> Option<char> {
    COMMENT_CHARS
        .into_iter()
        .find(|c| !message.lines().any(|line| line.starts_with(*c)))
}

fn check_trailer(trailer: &Trailer) -> Result<(), CommitRefused> {
    let key_ok = !trailer.key.is_empty()
        && trailer
            .key
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-');
    if !key_ok {
        return Err(CommitRefused::Trailer(format!(
            "the key {:?} must be letters, digits and '-'",
            trailer.key
        )));
    }
    if trailer.value.contains(['\n', '\r']) {
        return Err(CommitRefused::Trailer(format!(
            "the value of {} has more than one line",
            trailer.key
        )));
    }
    Ok(())
}

/// Commits the index with `message` and `trailers` and returns the new commit. Hooks run; theos
/// never skips them. Line endings of the message become LF and surrounding blank space goes;
/// lines starting with `#` stay in it. Not cancellable.
///
/// # Errors
///
/// [`CommitRefused::Message`] (also when no comment character is free), [`CommitRefused::Trailer`], and [`CommitRefused::Git`] when git
/// (or a hook) fails, or when the new commit cannot be read.
pub fn commit(
    git: &Git,
    top: &Path,
    message: &str,
    trailers: &[Trailer],
) -> Result<CommitId, CommitRefused> {
    let message = message.replace("\r\n", "\n");
    let message = message.trim();
    if message.is_empty() {
        return Err(CommitRefused::Message("it is empty".to_owned()));
    }
    trailers.iter().try_for_each(check_trailer)?;

    let Some(comment) = comment_char(message) else {
        return Err(CommitRefused::Message(
            "every one of # ; @ ! $ % ^ & | : starts a line".to_owned(),
        ));
    };
    let comment = format!("core.commentChar={comment}");
    let mut config = vec![comment.as_str()];
    config.extend(TRAILER_SETTINGS);
    let lines: Vec<String> = trailers
        .iter()
        .map(|t| format!("{}: {}", t.key, t.value))
        .collect();
    let mut flags = vec!["--cleanup=whitespace", "-F", "-"];
    for line in &lines {
        flags.push("--trailer");
        flags.push(line);
    }
    let out = write_with(
        git,
        top,
        &config,
        "commit",
        &flags,
        Some(message.as_bytes()),
        &Cancel::new(),
    )?;
    let args = write_args_with(&config, "commit", &flags);
    checked(&args, out)?;
    repo::head(git, top)?.ok_or_else(|| {
        CommitRefused::Git(GitError::Output {
            args: args_text(&args),
            message: "there is no HEAD after the commit".to_owned(),
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_refused_message_names_its_reason() {
        let all = COMMENT_CHARS.map(|c| format!("{c} x")).join("\n");
        let refused = commit_refusal("");
        assert_eq!(refused, "invalid commit message: it is empty");
        let refused = commit_refusal(&all);
        assert_eq!(
            refused,
            "invalid commit message: every one of # ; @ ! $ % ^ & | : starts a line"
        );
    }

    fn commit_refusal(message: &str) -> String {
        let dir = tempfile::tempdir().unwrap_or_else(|error| panic!("{error}"));
        let config = dir.path().join("gitconfig");
        std::fs::write(&config, "").unwrap_or_else(|error| panic!("{error}"));
        let config = config.to_string_lossy().into_owned();
        let git = Git::find()
            .unwrap_or_else(|error| panic!("{error}"))
            .with_env(&[("GIT_CONFIG_GLOBAL", &config), ("GIT_CONFIG_NOSYSTEM", "1")]);
        let repository = dir.path().join("repo");
        std::fs::create_dir(&repository).unwrap_or_else(|error| panic!("{error}"));
        let init = std::process::Command::new("git")
            .args(["init", "-q", "-b", "main"])
            .current_dir(&repository)
            .env("GIT_CONFIG_GLOBAL", &config)
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .output()
            .unwrap_or_else(|error| panic!("{error}"));
        assert!(init.status.success(), "{init:?}");
        match commit(&git, &repository, message, &[]) {
            Err(error) => error.to_string(),
            Ok(id) => panic!("committed {id:?}"),
        }
    }

    #[test]
    fn the_first_char_that_starts_no_line_is_chosen() {
        assert_eq!(comment_char("a\n# b"), Some(';'));
        assert_eq!(comment_char("plain"), Some('#'));
    }

    #[test]
    fn no_char_is_left_when_each_one_starts_a_line() {
        let all = COMMENT_CHARS.map(|c| format!("{c} x")).join("\n");
        assert_eq!(comment_char(&all), None);
    }
}
