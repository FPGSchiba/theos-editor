//! The git runner: starts the `git` program without a shell and returns its stdout.

use std::ffi::{OsStr, OsString};
use std::io::{ErrorKind, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

/// Why a git command could not give an answer.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum GitError {
    /// No `git` program was found.
    #[error("git was not found on the PATH")]
    NotFound,
    /// git ran and exited with a failure.
    #[error("git {} failed ({}): {stderr}", .args.join(" "), exit_text(*.code))]
    Failed {
        /// The arguments git was given.
        args: Vec<String>,
        /// The exit code, `None` when a signal ended the process.
        code: Option<i32>,
        /// Everything git wrote to stderr, lossily decoded and trimmed.
        stderr: String,
    },
    /// git could not be started or its pipes failed.
    #[error("could not run git: {source}")]
    Io {
        /// The underlying error.
        source: std::io::Error,
    },
    /// git succeeded but its answer cannot be used.
    #[error("git {} gave an unusable answer: {message}", .args.join(" "))]
    Output {
        /// The arguments git was given.
        args: Vec<String>,
        /// What is wrong with the answer.
        message: String,
    },
}

fn exit_text(code: Option<i32>) -> String {
    code.map_or_else(
        || "no exit code".to_owned(),
        |code| format!("exit code {code}"),
    )
}

/// The git command line, found once and run many times.
#[derive(Debug, Clone)]
pub struct Git {
    program: PathBuf,
    env: Vec<(String, String)>,
}

impl Git {
    /// Finds `git` on the `PATH`.
    ///
    /// # Errors
    ///
    /// [`GitError::NotFound`] when no `git` program is on the `PATH`, and [`GitError::Io`]
    /// when the current folder cannot be read.
    pub fn find() -> Result<Git, GitError> {
        let cwd = std::env::current_dir().map_err(|source| GitError::Io { source })?;
        find_in(std::env::var_os("PATH"), &cwd)
    }

    /// Adds variables to the environment of every command this runner starts.
    #[must_use]
    pub fn with_env(mut self, env: &[(&str, &str)]) -> Git {
        self.env.extend(
            env.iter()
                .map(|(key, value)| ((*key).to_owned(), (*value).to_owned())),
        );
        self
    }

    /// Runs git with `args` in `cwd` and returns its stdout.
    ///
    /// The runner's variables, then `env`, are added to the inherited environment. `stdin` is
    /// piped when given and closed otherwise. It is written while stdout and stderr are read,
    /// so an answer larger than a pipe does not block.
    ///
    /// # Errors
    ///
    /// [`GitError::Io`] when git cannot be started or its pipes fail, and
    /// [`GitError::Failed`] when it exits with a failure.
    pub fn run(
        &self,
        args: &[&OsStr],
        cwd: &Path,
        stdin: Option<&[u8]>,
        env: &[(&str, &str)],
    ) -> Result<Vec<u8>, GitError> {
        let mut command = Command::new(&self.program);
        command
            .args(args)
            .current_dir(cwd)
            .envs(self.env.iter().map(|(key, value)| (key, value)))
            .envs(env.iter().copied())
            .stdin(if stdin.is_some() {
                Stdio::piped()
            } else {
                Stdio::null()
            })
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        let mut child = command.spawn().map_err(|source| GitError::Io { source })?;
        let pipe = child.stdin.take();
        let (output, written) = std::thread::scope(|scope| {
            let writer = scope.spawn(move || match (pipe, stdin) {
                (Some(mut pipe), Some(bytes)) => match pipe.write_all(bytes) {
                    // git may exit before it reads everything; its exit status tells why.
                    Err(error) if error.kind() != ErrorKind::BrokenPipe => Err(error),
                    _ => Ok(()),
                },
                _ => Ok(()),
            });
            let output = child.wait_with_output();
            (output, writer.join())
        });
        let output = output.map_err(|source| GitError::Io { source })?;
        if !output.status.success() {
            return Err(GitError::Failed {
                args: args
                    .iter()
                    .map(|arg| arg.to_string_lossy().into_owned())
                    .collect(),
                code: output.status.code(),
                stderr: String::from_utf8_lossy(&output.stderr).trim().to_owned(),
            });
        }
        if let Ok(Err(source)) = written {
            return Err(GitError::Io { source });
        }
        Ok(output.stdout)
    }
}

/// Finds `git` in the search path `path`, a list separated as `PATH` is, with relative entries
/// resolved from `cwd`. With `None` there is nothing to search and the answer is `NotFound`.
fn find_in(path: Option<OsString>, cwd: &Path) -> Result<Git, GitError> {
    which::which_in("git", path, cwd)
        .map(|program| Git {
            program,
            env: Vec::new(),
        })
        .map_err(|_| GitError::NotFound)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_empty_path_finds_no_git() -> Result<(), Box<dyn std::error::Error>> {
        let cwd = tempfile::tempdir()?;
        let result = find_in(Some(OsString::new()), cwd.path());
        assert!(matches!(result, Err(GitError::NotFound)));
        Ok(())
    }

    #[test]
    fn a_missing_program_is_io() -> Result<(), Box<dyn std::error::Error>> {
        let cwd = tempfile::tempdir()?;
        let git = Git {
            program: cwd.path().join("no-such-git"),
            env: Vec::new(),
        };
        let result = git.run(&[OsStr::new("--version")], cwd.path(), None, &[]);
        assert!(matches!(result, Err(GitError::Io { .. })));
        Ok(())
    }
}
