//! The git runner: starts the `git` program without a shell and returns its stdout.

use std::ffi::{OsStr, OsString};
use std::io::{ErrorKind, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread::ScopedJoinHandle;
use std::time::Instant;

use crate::tree::Tree;
use crate::wait::{self, Waited};
use crate::{Cancel, GitVersion};

/// Why a git command could not give an answer.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum GitError {
    /// No `git` program was found.
    #[error(
        "git was not found on the PATH. theos needs git {minimum} or newer: install it from https://git-scm.com/downloads, then open a new terminal or restart the editor.",
        minimum = MinimumShort
    )]
    NotFound,
    /// The `git` found is older than [`GitVersion::MINIMUM`].
    #[error(
        "git {found} ({}) is too old. theos needs git {} or newer: update it from https://git-scm.com/downloads.",
        .program.display(),
        MinimumShort
    )]
    TooOld {
        /// The `git` program that was found.
        program: PathBuf,
        /// The version it reported.
        found: GitVersion,
        /// The oldest version theos runs with.
        required: GitVersion,
    },
    /// The command was cancelled before it finished.
    #[error("git {} was cancelled", .args.join(" "))]
    Cancelled {
        /// The arguments git was given.
        args: Vec<String>,
    },
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

/// Displays [`GitVersion::MINIMUM`] as `major.minor`, for the messages.
struct MinimumShort;

impl std::fmt::Display for MinimumShort {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let GitVersion { major, minor, .. } = GitVersion::MINIMUM;
        write!(f, "{major}.{minor}")
    }
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

/// git's whole answer to a command that ran to its end, whatever its exit code.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Output {
    /// The exit code, `None` when a signal ended the process.
    pub code: Option<i32>,
    /// Everything git wrote to stdout.
    pub stdout: Vec<u8>,
    /// Everything git wrote to stderr, lossily decoded and trimmed.
    pub stderr: String,
}

impl Output {
    /// Whether git exited with code 0.
    #[must_use]
    pub fn success(&self) -> bool {
        self.code == Some(0)
    }
}

/// The git command line, found once and run many times.
#[derive(Debug, Clone)]
pub struct Git {
    program: PathBuf,
    env: Vec<(OsString, OsString)>,
    version: GitVersion,
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
    /// Then it runs `git --version` once, in the current folder with that environment, and
    /// refuses a git older than [`GitVersion::MINIMUM`].
    ///
    /// # Errors
    ///
    /// [`GitError::NotFound`] when no `git` program is on the `PATH`, [`GitError::Io`] when the
    /// current folder cannot be read or git cannot be started, [`GitError::TooOld`] when git is
    /// older than the minimum, and [`GitError::Output`] when its version cannot be read.
    pub fn find() -> Result<Git, GitError> {
        let cwd = std::env::current_dir().map_err(|source| GitError::Io { source })?;
        let program = find_in(std::env::var_os("PATH"), &cwd)?;
        let mut git = Git {
            program,
            env: resolved_env(|name| std::env::var_os(name), &cwd),
            version: GitVersion::MINIMUM,
        };
        let answer = git.run(&[OsStr::new("--version")], &cwd, None, &[])?;
        git.version = checked(git.program.clone(), &answer)?;
        Ok(git)
    }

    /// The version `find` read.
    #[must_use]
    pub fn version(&self) -> GitVersion {
        self.version
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
        self.run_cancellable(args, cwd, stdin, env, &Cancel::new())
    }

    /// Runs git as [`Git::run`] does, and stops git with every process it started when `cancel`
    /// is set.
    ///
    /// A flag set before the start returns [`GitError::Cancelled`] without starting git. While
    /// git runs, the flag is checked every few milliseconds; when it is set, git's whole process
    /// tree is stopped (Windows: the job git runs in is closed; Unix: git's process group gets
    /// `SIGKILL`), git is waited for, and the answer is [`GitError::Cancelled`].
    ///
    /// # Errors
    ///
    /// As [`Git::run`], and [`GitError::Cancelled`] when `cancel` is set before git exits.
    pub fn run_cancellable(
        &self,
        args: &[&OsStr],
        cwd: &Path,
        stdin: Option<&[u8]>,
        env: &[(&str, &str)],
        cancel: &Cancel,
    ) -> Result<Vec<u8>, GitError> {
        let output = self.output(args, cwd, stdin, env, cancel)?;
        if output.success() {
            return Ok(output.stdout);
        }
        let error = GitError::Failed {
            args: args_text(args),
            code: output.code,
            stderr: output.stderr,
        };
        // A failure exit is often expected (`rev-parse --verify --quiet` of a missing
        // version, `config --get` of an unset key); the caller decides whether it matters.
        tracing::debug!(?args, cwd = %cwd.display(), %error, "git failed");
        Err(error)
    }

    /// Runs git as [`Git::run_cancellable`] does and returns its whole answer: the exit code,
    /// stdout and stderr, whether git succeeded or failed.
    ///
    /// # Errors
    ///
    /// [`GitError::Cancelled`] when `cancel` is set before git exits, and [`GitError::Io`] when
    /// git cannot be started or its pipes fail. A failure exit is an [`Output`], not an error.
    pub fn output(
        &self,
        args: &[&OsStr],
        cwd: &Path,
        stdin: Option<&[u8]>,
        env: &[(&str, &str)],
        cancel: &Cancel,
    ) -> Result<Output, GitError> {
        let started = Instant::now();
        let result = self.spawn_and_wait(args, cwd, stdin, env, cancel);
        let (elapsed, cwd) = (started.elapsed(), cwd.display());
        match &result {
            Ok(output) => tracing::debug!(?args, %cwd, ?elapsed, code = ?output.code, "git"),
            Err(GitError::Cancelled { .. }) => {
                tracing::debug!(?args, %cwd, ?elapsed, "git cancelled");
            }
            Err(error) => tracing::warn!(?args, %cwd, ?elapsed, %error, "git could not run"),
        }
        result
    }

    /// The body of [`Git::output`], without the logging.
    fn spawn_and_wait(
        &self,
        args: &[&OsStr],
        cwd: &Path,
        stdin: Option<&[u8]>,
        env: &[(&str, &str)],
        cancel: &Cancel,
    ) -> Result<Output, GitError> {
        if cancel.is_cancelled() {
            return Err(GitError::Cancelled {
                args: args_text(args),
            });
        }
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
        let (tree, mut child) =
            Tree::spawn(&mut command).map_err(|source| GitError::Io { source })?;
        let pipe = child.stdin.take();
        let stdout = child.stdout.take();
        let stderr = child.stderr.take();
        let (waited, stdout, stderr, written) = std::thread::scope(|scope| {
            let writer = scope.spawn(move || match (pipe, stdin) {
                (Some(mut pipe), Some(bytes)) => match pipe.write_all(bytes) {
                    // git may exit before it reads everything; its exit status tells why.
                    Err(error) if error.kind() != ErrorKind::BrokenPipe => Err(error),
                    _ => Ok(()),
                },
                _ => Ok(()),
            });
            let stdout = scope.spawn(move || read_all(stdout));
            let stderr = scope.spawn(move || read_all(stderr));
            let waited = wait::wait(scope, child, tree, cancel);
            (waited, joined(stdout), joined(stderr), joined(writer))
        });
        let io = |source| GitError::Io { source };
        let status = match waited.map_err(io)? {
            Waited::Cancelled => {
                return Err(GitError::Cancelled {
                    args: args_text(args),
                });
            }
            Waited::Exited(status) => status,
        };
        let stdout = stdout.map_err(io)?;
        let stderr = stderr.map_err(io)?;
        if status.success() {
            written.map_err(io)?;
        }
        Ok(Output {
            code: status.code(),
            stdout,
            stderr: String::from_utf8_lossy(&stderr).trim().to_owned(),
        })
    }
}

/// Reads a pipe of git's to its end; no pipe reads as nothing.
fn read_all(pipe: Option<impl Read>) -> std::io::Result<Vec<u8>> {
    let mut bytes = Vec::new();
    if let Some(mut pipe) = pipe {
        pipe.read_to_end(&mut bytes)?;
    }
    Ok(bytes)
}

/// The answer of a pipe thread; a thread that panicked is an I/O error.
fn joined<T>(thread: ScopedJoinHandle<'_, std::io::Result<T>>) -> std::io::Result<T> {
    thread
        .join()
        .map_err(|_| std::io::Error::other("a thread that moves git's bytes panicked"))?
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
fn find_in(path: Option<OsString>, cwd: &Path) -> Result<PathBuf, GitError> {
    which::which_in("git", path, cwd).map_err(|_| GitError::NotFound)
}

/// Reads the answer of `git --version` of `program` and refuses a version below the minimum.
fn checked(program: PathBuf, answer: &[u8]) -> Result<GitVersion, GitError> {
    let text = String::from_utf8_lossy(answer);
    let text = text.trim();
    let found = GitVersion::parse(text).ok_or_else(|| GitError::Output {
        args: vec!["--version".to_owned()],
        message: format!("cannot read a version from {text:?}"),
    })?;
    if found < GitVersion::MINIMUM {
        return Err(GitError::TooOld {
            program,
            found,
            required: GitVersion::MINIMUM,
        });
    }
    Ok(found)
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

    fn version(major: u32, minor: u32, patch: u32) -> GitVersion {
        GitVersion {
            major,
            minor,
            patch,
        }
    }

    // AC: CE-21.3
    #[test]
    fn a_version_below_the_minimum_is_too_old() {
        let program = PathBuf::from("git");
        let result = checked(program.clone(), b"git version 2.37.9\n");
        assert!(matches!(
            result,
            Err(GitError::TooOld { program: p, found, required })
                if p == program && found == version(2, 37, 9) && required == GitVersion::MINIMUM
        ));
    }

    // AC: CE-21.3
    #[test]
    fn the_minimum_version_is_accepted() {
        let result = checked(PathBuf::from("git"), b"git version 2.38.0\n");
        assert!(matches!(result, Ok(found) if found == version(2, 38, 0)));
    }

    // AC: CE-21.3
    #[test]
    fn an_unreadable_answer_is_output_with_the_answer() {
        let result = checked(PathBuf::from("git"), b"nonsense");
        assert!(
            matches!(result, Err(GitError::Output { message, .. }) if message.contains("nonsense"))
        );
    }

    // AC: CE-21.3
    #[test]
    fn not_found_tells_what_to_do() {
        assert_eq!(
            GitError::NotFound.to_string(),
            "git was not found on the PATH. theos needs git 2.38 or newer: install it from \
             https://git-scm.com/downloads, then open a new terminal or restart the editor."
        );
    }

    // AC: CE-21.3
    #[test]
    fn too_old_tells_the_version_and_the_program() {
        let error = GitError::TooOld {
            program: PathBuf::from("/usr/bin/git"),
            found: version(2, 34, 1),
            required: GitVersion::MINIMUM,
        };
        assert_eq!(
            error.to_string(),
            format!(
                "git 2.34.1 ({}) is too old. theos needs git 2.38 or newer: update it from \
                 https://git-scm.com/downloads.",
                Path::new("/usr/bin/git").display()
            )
        );
    }

    #[test]
    fn cancelled_names_the_command() {
        let error = GitError::Cancelled {
            args: vec!["log".to_owned(), "--follow".to_owned()],
        };
        assert_eq!(error.to_string(), "git log --follow was cancelled");
    }

    #[test]
    fn a_missing_program_is_io() -> Result<(), Box<dyn std::error::Error>> {
        let cwd = tempfile::tempdir()?;
        let git = Git {
            program: cwd.path().join("no-such-git"),
            env: Vec::new(),
            version: GitVersion::MINIMUM,
        };
        let result = git.run(&[OsStr::new("--version")], cwd.path(), None, &[]);
        assert!(matches!(result, Err(GitError::Io { .. })));
        Ok(())
    }

    #[test]
    fn a_process_without_an_exit_code_is_named_so() {
        assert_eq!(exit_text(None), "no exit code");
        assert_eq!(exit_text(Some(3)), "exit code 3");
    }
}
