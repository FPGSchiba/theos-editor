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

/// The arguments as text, for error messages.
pub(crate) fn args_text(args: &[&OsStr]) -> Vec<String> {
    args.iter()
        .map(|arg| arg.to_string_lossy().into_owned())
        .collect()
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
    env: Vec<(OsString, OsString)>,
}

impl Git {
    /// Finds `git` on the `PATH`.
    ///
    /// It also resolves, once, the repository that git's variables name in the current folder,
    /// so that every command the runner starts sees that repository from any folder it runs in,
    /// such as a project folder below the top. This assumes theos runs where git started it: in
    /// a hook, which git starts in the top folder of the work tree, or in a user's shell.
    ///
    /// - A relative `GIT_DIR`, `GIT_WORK_TREE`, `GIT_COMMON_DIR` or `GIT_OBJECT_DIRECTORY` is
    ///   relative to the current folder, as git documents, so the runner sets it to the current
    ///   folder joined with it. git hands hooks such values: `GIT_WORK_TREE=.` to the hooks of a
    ///   commit that was given its work tree, `GIT_DIR=.git` to those of `git --git-dir=.git`.
    /// - When `GIT_DIR` is set and `GIT_WORK_TREE` is not, git takes the current folder as the
    ///   top of the work tree, unless `core.worktree` names one. The runner sets `GIT_WORK_TREE`
    ///   to the current folder, and the variable outranks `core.worktree`. In a hook that is the
    ///   same folder, as git starts the hook in the top it found, `core.worktree` included; in a
    ///   shell, theos takes the current folder over `core.worktree`. git hands the hooks of a
    ///   linked work tree, or of a repository made with `--separate-git-dir`, an absolute
    ///   `GIT_DIR` alone.
    /// - `GIT_INDEX_FILE` is left as it is, since git resolves it in the top, where every read of
    ///   the index runs; so is the list `GIT_ALTERNATE_OBJECT_DIRECTORIES`.
    ///
    /// # Errors
    ///
    /// [`GitError::NotFound`] when no `git` program is on the `PATH`, and [`GitError::Io`]
    /// when the current folder cannot be read.
    pub fn find() -> Result<Git, GitError> {
        let cwd = std::env::current_dir().map_err(|source| GitError::Io { source })?;
        let mut git = find_in(std::env::var_os("PATH"), &cwd)?;
        git.env
            .extend(resolved_env(|name| std::env::var_os(name), &cwd));
        Ok(git)
    }

    /// Adds variables to the environment of every command this runner starts.
    #[must_use]
    pub fn with_env(mut self, env: &[(&str, &str)]) -> Git {
        self.env.extend(
            env.iter()
                .map(|(key, value)| (OsString::from(key), OsString::from(value))),
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
                args: args_text(args),
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

/// The variable that names the git folder.
const GIT_DIR: &str = "GIT_DIR";

/// The variable that names the top of the work tree.
const GIT_WORK_TREE: &str = "GIT_WORK_TREE";

/// The variables that name one path, which git reads relative to the current folder.
const PATHS: [&str; 4] = [
    GIT_DIR,
    GIT_WORK_TREE,
    "GIT_COMMON_DIR",
    "GIT_OBJECT_DIRECTORY",
];

/// The variables to add to the runner's environment so that every command it starts, from any
/// folder, sees the repository that git's variables name in a process whose current folder is
/// `cwd`. `value` reads one of those variables.
///
/// - Each variable of `PATHS` that holds a relative path is set to `cwd` joined with it.
/// - When `GIT_DIR` is set and `GIT_WORK_TREE` is not, `GIT_WORK_TREE` is set to `cwd`.
fn resolved_env(value: impl Fn(&str) -> Option<OsString>, cwd: &Path) -> Vec<(OsString, OsString)> {
    let mut env: Vec<(OsString, OsString)> = PATHS
        .into_iter()
        .filter_map(|name| {
            let path = PathBuf::from(value(name)?);
            path.is_relative()
                .then(|| (OsString::from(name), cwd.join(path).into_os_string()))
        })
        .collect();
    if value(GIT_DIR).is_some() && value(GIT_WORK_TREE).is_none() {
        env.push((OsString::from(GIT_WORK_TREE), cwd.as_os_str().to_owned()));
    }
    env
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

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    /// The variables to add for an environment that holds exactly `set`, in the folder `cwd`.
    fn resolved(set: &[(&str, &OsStr)], cwd: &Path) -> Vec<(OsString, OsString)> {
        resolved_env(
            |name| {
                set.iter()
                    .find(|(key, _)| *key == name)
                    .map(|(_, value)| value.to_os_string())
            },
            cwd,
        )
    }

    /// A pair to add, as the helper answers it.
    fn pair(name: &str, value: &Path) -> (OsString, OsString) {
        (OsString::from(name), value.as_os_str().to_owned())
    }

    /// An absolute folder on every platform, as a hook's current folder is.
    fn top() -> Result<PathBuf, std::io::Error> {
        Ok(std::env::current_dir()?.join("top"))
    }

    #[test]
    fn no_variables_add_nothing() -> TestResult {
        assert_eq!(resolved(&[], &top()?), Vec::new());
        Ok(())
    }

    #[test]
    fn a_relative_path_variable_is_joined_to_the_current_folder() -> TestResult {
        let cwd = top()?;
        for name in ["GIT_WORK_TREE", "GIT_COMMON_DIR", "GIT_OBJECT_DIRECTORY"] {
            let value = Path::new("up").join("there");
            assert_eq!(
                resolved(&[(name, value.as_os_str())], &cwd),
                vec![pair(name, &cwd.join(&value))],
                "{name}"
            );
        }
        Ok(())
    }

    #[test]
    fn an_absolute_path_variable_is_left_as_it_is() -> TestResult {
        let cwd = top()?;
        let elsewhere = cwd.join("elsewhere");
        for name in ["GIT_WORK_TREE", "GIT_COMMON_DIR", "GIT_OBJECT_DIRECTORY"] {
            assert_eq!(
                resolved(&[(name, elsewhere.as_os_str())], &cwd),
                Vec::new(),
                "{name}"
            );
        }
        Ok(())
    }

    #[test]
    fn an_absolute_git_dir_alone_makes_the_current_folder_the_work_tree() -> TestResult {
        let cwd = top()?;
        let git_dir = cwd.join(".git").join("worktrees").join("linked");
        assert_eq!(
            resolved(&[("GIT_DIR", git_dir.as_os_str())], &cwd),
            vec![pair("GIT_WORK_TREE", &cwd)]
        );
        Ok(())
    }

    #[test]
    fn a_relative_git_dir_alone_is_joined_and_makes_the_current_folder_the_work_tree() -> TestResult
    {
        // `git --git-dir=.git commit` gives its hook `GIT_DIR=.git` and no `GIT_WORK_TREE`.
        let cwd = top()?;
        assert_eq!(
            resolved(&[("GIT_DIR", OsStr::new(".git"))], &cwd),
            vec![
                pair("GIT_DIR", &cwd.join(".git")),
                pair("GIT_WORK_TREE", &cwd)
            ]
        );
        Ok(())
    }

    #[test]
    fn an_absolute_git_dir_with_a_relative_work_tree_joins_the_work_tree() -> TestResult {
        // `git --git-dir=<absolute> --work-tree=<absolute> commit` gives its hook
        // `GIT_WORK_TREE=.`.
        let cwd = top()?;
        let git_dir = cwd.join(".git");
        assert_eq!(
            resolved(
                &[
                    ("GIT_DIR", git_dir.as_os_str()),
                    ("GIT_WORK_TREE", OsStr::new("."))
                ],
                &cwd
            ),
            vec![pair("GIT_WORK_TREE", &cwd.join("."))]
        );
        Ok(())
    }

    #[test]
    fn a_relative_git_dir_and_work_tree_are_both_joined() -> TestResult {
        // `GIT_WORK_TREE=<absolute> git commit` gives its hook `GIT_DIR=.git` and
        // `GIT_WORK_TREE=.`.
        let cwd = top()?;
        assert_eq!(
            resolved(
                &[
                    ("GIT_DIR", OsStr::new(".git")),
                    ("GIT_WORK_TREE", OsStr::new("."))
                ],
                &cwd
            ),
            vec![
                pair("GIT_DIR", &cwd.join(".git")),
                pair("GIT_WORK_TREE", &cwd.join("."))
            ]
        );
        Ok(())
    }

    #[test]
    fn absolute_git_dir_and_work_tree_add_nothing() -> TestResult {
        let cwd = top()?;
        let git_dir = cwd.join(".git");
        let elsewhere = cwd.join("elsewhere");
        assert_eq!(
            resolved(
                &[
                    ("GIT_DIR", git_dir.as_os_str()),
                    ("GIT_WORK_TREE", elsewhere.as_os_str())
                ],
                &cwd
            ),
            Vec::new()
        );
        Ok(())
    }

    #[test]
    fn every_relative_path_variable_is_joined_together() -> TestResult {
        let cwd = top()?;
        let set = [
            ("GIT_DIR", OsStr::new("repo.git")),
            ("GIT_WORK_TREE", OsStr::new("tree")),
            ("GIT_COMMON_DIR", OsStr::new("common")),
            ("GIT_OBJECT_DIRECTORY", OsStr::new("objects")),
        ];
        let joined: Vec<_> = set
            .iter()
            .map(|(name, value)| pair(name, &cwd.join(value)))
            .collect();
        assert_eq!(resolved(&set, &cwd), joined);
        Ok(())
    }

    #[test]
    fn the_index_file_and_the_alternates_are_left_as_they_are() -> TestResult {
        let cwd = top()?;
        let set = [
            ("GIT_INDEX_FILE", OsStr::new(".git/other-index")),
            ("GIT_ALTERNATE_OBJECT_DIRECTORIES", OsStr::new("objects")),
        ];
        assert_eq!(resolved(&set, &cwd), Vec::new());
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
