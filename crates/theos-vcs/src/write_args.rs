//! The one list of options every git write starts with, and the one place a failing write is
//! turned into an error.

use std::ffi::OsStr;
use std::path::Path;

use crate::git::{args_text, failed};
use crate::{Cancel, Git, GitError, Output};

/// The options that come before the command of every write: literal pathspecs, no colour or
/// quoting of paths, and no automatic housekeeping (`gc`, `maintenance`) running behind a write.
/// There is no `--no-optional-locks`: a write takes the locks it needs.
const OPTIONS: [&str; 9] = [
    "--literal-pathspecs",
    "-c",
    "color.ui=never",
    "-c",
    "core.quotePath=false",
    "-c",
    "gc.auto=0",
    "-c",
    "maintenance.auto=false",
];

/// The arguments of a write: the write options, then `command`, then `flags`.
pub(crate) fn write_args<'a>(command: &'a str, flags: &[&'a str]) -> Vec<&'a OsStr> {
    OPTIONS
        .iter()
        .copied()
        .chain(std::iter::once(command))
        .chain(flags.iter().copied())
        .map(OsStr::new)
        .collect()
}

/// Runs a write in `top` and returns git's whole answer, whatever its exit code.
///
/// # Errors
///
/// [`GitError::Cancelled`] when `cancel` is set, and [`GitError::Io`] when git cannot be run.
pub(crate) fn write(
    git: &Git,
    top: &Path,
    command: &str,
    flags: &[&str],
    stdin: Option<&[u8]>,
    cancel: &Cancel,
) -> Result<Output, GitError> {
    git.output(&write_args(command, flags), top, stdin, &[], cancel)
}

/// Passes a successful answer; turns a failure into [`GitError::IndexLocked`] when its stderr
/// names `index.lock` (the one message theos looks into) and into [`GitError::Failed`] otherwise.
///
/// # Errors
///
/// The two errors above.
pub(crate) fn checked(args: &[&OsStr], out: Output) -> Result<Output, GitError> {
    if out.success() {
        return Ok(out);
    }
    if out.stderr.contains("index.lock") {
        tracing::debug!(args = ?args_text(args), "git found an index.lock");
        return Err(GitError::IndexLocked { stderr: out.stderr });
    }
    Err(failed(args, out))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Cancel, Git, GitError};

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    #[test]
    fn the_options_come_first_then_the_command_then_its_flags() {
        let args = write_args("add", &["-A", "--", "a"]);
        let text: Vec<&str> = args.iter().filter_map(|arg| arg.to_str()).collect();
        assert_eq!(
            text,
            [
                "--literal-pathspecs",
                "-c",
                "color.ui=never",
                "-c",
                "core.quotePath=false",
                "-c",
                "gc.auto=0",
                "-c",
                "maintenance.auto=false",
                "add",
                "-A",
                "--",
                "a",
            ]
        );
    }

    #[test]
    fn a_held_index_lock_is_index_locked_and_the_file_stays() -> TestResult {
        let top = tempfile::tempdir()?;
        let config = top.path().join("empty-gitconfig");
        std::fs::write(&config, "")?;
        let config = config.to_string_lossy().into_owned();
        let git = Git::find()?.with_env(&[
            ("GIT_CONFIG_GLOBAL", config.as_str()),
            ("GIT_CONFIG_NOSYSTEM", "1"),
        ]);
        let cancel = Cancel::new();
        let init = git.output(
            &[std::ffi::OsStr::new("init")],
            top.path(),
            None,
            &[],
            &cancel,
        )?;
        assert!(init.success(), "{}", init.stderr);
        let lock = top.path().join(".git").join("index.lock");
        std::fs::write(&lock, "")?;

        let out = write(&git, top.path(), "add", &["-A"], None, &cancel)?;
        let args = write_args("add", &["-A"]);
        let result = checked(&args, out);

        match result {
            Err(GitError::IndexLocked { stderr }) => assert!(stderr.contains("index.lock")),
            other => panic!("expected IndexLocked, got {other:?}"),
        }
        assert!(lock.exists());
        Ok(())
    }

    #[test]
    fn another_failure_is_failed_and_a_success_passes() -> TestResult {
        let top = tempfile::tempdir()?;
        let git = Git::find()?;
        let cancel = Cancel::new();
        let out = write(&git, top.path(), "no-such-command", &[], None, &cancel)?;
        let args = write_args("no-such-command", &[]);
        assert!(matches!(checked(&args, out), Err(GitError::Failed { .. })));
        let ok = crate::Output {
            code: Some(0),
            stdout: b"x".to_vec(),
            stderr: String::new(),
        };
        assert_eq!(checked(&args, ok)?.stdout, b"x");
        Ok(())
    }

    #[test]
    fn index_locked_tells_what_to_do() {
        let error = GitError::IndexLocked {
            stderr: "boom".to_owned(),
        };
        assert_eq!(
            error.to_string(),
            "another git is working in this repository (its index.lock exists): boom"
        );
    }
}
