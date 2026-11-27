//! The status of a repository, with renames that are not staged yet paired by git.
//!
//! git's own status pairs a deleted file with a new one only when both are in the index. To pair
//! a record renamed on disk (`mother-ilse.md` → `mother-ilsa.md`), the untracked files are added
//! with `git add --intent-to-add` to a temporary copy of the index, and the status is read from
//! that copy; the real index is never written.

pub(crate) mod parse;

use std::collections::HashMap;
use std::ffi::OsStr;
use std::path::{Path, PathBuf};

use crate::Cancel;
use crate::git::{Git, GitError, args_text};
use crate::read_args::read_args;
use crate::staged::parse::fields;

/// The status of a repository: its branch and every path that differs.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Status {
    /// The checked-out branch.
    pub branch: Branch,
    /// The changed, renamed, conflicted and untracked paths, in git's order.
    pub entries: Vec<Entry>,
}

/// The checked-out branch.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Branch {
    /// The branch name; `None` on a detached HEAD.
    pub head: Option<String>,
    /// The commit; `None` before the first commit.
    pub commit: Option<String>,
    /// The upstream and how far the branch is ahead and behind it, when there is one.
    pub upstream: Option<Upstream>,
}

/// The upstream of a branch.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Upstream {
    /// The upstream's name, such as `origin/main`.
    pub name: String,
    /// Commits on the branch that the upstream lacks.
    pub ahead: u32,
    /// Commits on the upstream that the branch lacks.
    pub behind: u32,
}

/// One path of the status. Paths are git's bytes, relative to the top, with `/`.
#[non_exhaustive]
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Entry {
    /// A tracked path changed in the index, in the working tree, or both (`1` lines).
    Changed {
        /// The path.
        path: Vec<u8>,
        /// The change from `HEAD` to the index.
        staged: Change,
        /// The change from the index to the working tree.
        unstaged: Change,
    },
    /// A rename (or copy) git paired, in the index or in the working tree (`2` lines).
    Renamed {
        /// The path before.
        from: Vec<u8>,
        /// The path after.
        to: Vec<u8>,
        /// The change from `HEAD` to the index.
        staged: Change,
        /// The change from the index to the working tree.
        unstaged: Change,
        /// How similar the two files are, from 0 to 100.
        score: u8,
    },
    /// A path with a merge conflict (`u` lines).
    Conflicted {
        /// The path.
        path: Vec<u8>,
        /// Which sides changed it.
        conflict: Conflict,
    },
    /// A file git does not track and does not ignore.
    Untracked {
        /// The path; a nested repository ends with `/`, as git spells it.
        path: Vec<u8>,
    },
}

/// One side of a change: `HEAD` to the index, or the index to the working tree.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Change {
    /// No change (`.`).
    Unchanged,
    /// The content changed (`M`).
    Modified,
    /// The kind of file changed, such as a file to a link (`T`).
    TypeChanged,
    /// Added (`A`).
    Added,
    /// Deleted (`D`).
    Deleted,
    /// Renamed (`R`).
    Renamed,
    /// Copied (`C`).
    Copied,
}

/// The kind of a merge conflict, as git's `XY` names it.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Conflict {
    /// `DD`: deleted on both sides.
    BothDeleted,
    /// `AU`: added by us.
    AddedByUs,
    /// `UD`: deleted by them.
    DeletedByThem,
    /// `UA`: added by them.
    AddedByThem,
    /// `DU`: deleted by us.
    DeletedByUs,
    /// `AA`: added on both sides.
    BothAdded,
    /// `UU`: modified on both sides.
    BothModified,
}

/// The variable that names the index git reads and writes.
const GIT_INDEX_FILE: &str = "GIT_INDEX_FILE";

/// Reads the status of the repository whose top folder is `top`.
///
/// Renames that are not staged yet are paired: a file deleted on disk and an untracked file git
/// finds similar are one [`Entry::Renamed`] with `unstaged: Renamed`. The untracked files are
/// added with intent to add to a temporary copy of the index (the one the runner's environment
/// names, such as a hook's `GIT_INDEX_FILE`), which is deleted when `status` returns, also on an
/// error or a cancel; the real index is never written. A file the user added with `git add -N`
/// stays [`Entry::Changed`] with `unstaged: Added`, as git shows it. Ignored files are not listed;
/// untracked files are listed one by one, also inside an untracked folder. An untracked file that
/// vanishes while `status` runs (an editor's save through a temporary file) is left out; when
/// adding the untracked files still fails, they are listed again and the adding is repeated once.
///
/// # Errors
///
/// The errors of [`Git::run_cancellable`], [`GitError::Cancelled`] when `cancel` is set,
/// [`GitError::Output`] when an answer of git cannot be read, and [`GitError::Io`] when the
/// temporary index cannot be made.
pub fn status(git: &Git, top: &Path, cancel: &Cancel) -> Result<Status, GitError> {
    status_in(git, top, cancel, &std::env::temp_dir())
}

/// [`status`] with the temporary index made in the folder `temp`.
pub(crate) fn status_in(
    git: &Git,
    top: &Path,
    cancel: &Cancel,
    temp: &Path,
) -> Result<Status, GitError> {
    let untracked = untracked_files(git, top, cancel)?;
    status_from(git, top, cancel, temp, untracked)
}

/// [`status_in`] from the untracked files `untracked` that step 1 listed.
///
/// A file may vanish or change after it was listed (an editor saves through a temporary file
/// and a rename), and `add` fails on a path that no longer matches. So a failed `add` is
/// repeated once from step 1 with a fresh list; a second failure is the error.
fn status_from(
    git: &Git,
    top: &Path,
    cancel: &Cancel,
    temp: &Path,
    untracked: Vec<Vec<u8>>,
) -> Result<Status, GitError> {
    match attempt(git, top, cancel, temp, untracked)? {
        Attempt::Done(status) => Ok(status),
        Attempt::AddFailed(error) => {
            tracing::debug!(%error, "the untracked files changed; reading them again");
            match attempt(git, top, cancel, temp, untracked_files(git, top, cancel)?)? {
                Attempt::Done(status) => Ok(status),
                Attempt::AddFailed(error) => Err(error),
            }
        }
    }
}

/// How one attempt of steps 2 to 5 ended.
enum Attempt {
    /// The status.
    Done(Status),
    /// `add --intent-to-add` failed, most likely because an untracked file changed.
    AddFailed(GitError),
}

/// Steps 2 to 5 of [`status`] with the untracked files `untracked`; those gone from disk by now
/// are left out first.
fn attempt(
    git: &Git,
    top: &Path,
    cancel: &Cancel,
    temp: &Path,
    untracked: Vec<Vec<u8>>,
) -> Result<Attempt, GitError> {
    let untracked = existing(top, untracked);
    if untracked.is_empty() {
        return porcelain(git, top, cancel, &[]).map(Attempt::Done);
    }
    let folder = tempfile::Builder::new()
        .prefix("theos-status-")
        .tempdir_in(temp)
        .map_err(|source| GitError::Io { source })?;
    let copy = folder.path().join("index");
    copy_index(&index_file(git, top, cancel)?, &copy)?;
    let copy = copy.to_str().ok_or_else(|| GitError::Io {
        source: std::io::Error::other("the temporary folder's path is not UTF-8"),
    })?;
    let env = [(GIT_INDEX_FILE, copy)];
    match intend_to_add(git, top, cancel, &env, &untracked) {
        Ok(()) => {}
        Err(error @ GitError::Failed { .. }) => return Ok(Attempt::AddFailed(error)),
        Err(error) => return Err(error),
    }
    let status = porcelain(git, top, cancel, &env)?;
    Ok(Attempt::Done(paired(status, &untracked)))
}

/// Step 1: the files git does not track and does not ignore, in git's spelling.
fn untracked_files(git: &Git, top: &Path, cancel: &Cancel) -> Result<Vec<Vec<u8>>, GitError> {
    query(
        git,
        top,
        cancel,
        &read_args("ls-files", &["--others", "--exclude-standard", "-z"]),
        None,
        &[],
        |out| Ok(fields(out)?.into_iter().map(<[u8]>::to_vec).collect()),
    )
}

/// The paths of `paths` that still exist under `top`. A path that is not UTF-8 is kept; if it is
/// gone, `add` fails and the attempt is repeated.
fn existing(top: &Path, paths: Vec<Vec<u8>>) -> Vec<Vec<u8>> {
    paths
        .into_iter()
        .filter(|path| match std::str::from_utf8(path) {
            Ok(text) => !matches!(
                std::fs::symlink_metadata(top.join(text)),
                Err(error) if error.kind() == std::io::ErrorKind::NotFound
            ),
            Err(_) => true,
        })
        .collect()
}

/// Runs git with `args` in `top` and reads its answer with `parse`, naming the command when it is
/// unusable.
fn query<T>(
    git: &Git,
    top: &Path,
    cancel: &Cancel,
    args: &[&OsStr],
    stdin: Option<&[u8]>,
    env: &[(&str, &str)],
    parse: impl FnOnce(&[u8]) -> Result<T, String>,
) -> Result<T, GitError> {
    let out = git.run_cancellable(args, top, stdin, env, cancel)?;
    parse(&out).map_err(|message| GitError::Output {
        args: args_text(args),
        message,
    })
}

/// The index file git reads, absolute: `rev-parse --git-path index` honours `GIT_INDEX_FILE` and
/// may answer a path relative to `top`.
fn index_file(git: &Git, top: &Path, cancel: &Cancel) -> Result<PathBuf, GitError> {
    let args = read_args("rev-parse", &["--git-path", "index"]);
    let path = query(git, top, cancel, &args, None, &[], |out| {
        let text = std::str::from_utf8(out).map_err(|_| "the path is not UTF-8")?;
        let text = text.trim_end_matches(['\n', '\r']);
        if text.is_empty() {
            return Err("the answer is empty".to_owned());
        }
        Ok(PathBuf::from(text))
    })?;
    Ok(top.join(path))
}

/// Copies the index file `index` to `copy`; a repository without an index file copies nothing,
/// as git reads a missing index as empty.
fn copy_index(index: &Path, copy: &Path) -> Result<(), GitError> {
    match std::fs::copy(index, copy) {
        Ok(_) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(source) => Err(GitError::Io { source }),
    }
}

/// Adds `paths` with intent to add to the index `env` names; the paths go on stdin, so no command
/// line grows with them. The copy is written whole (`core.splitIndex=false`), so no shared index
/// file is added to the repository.
fn intend_to_add(
    git: &Git,
    top: &Path,
    cancel: &Cancel,
    env: &[(&str, &str)],
    paths: &[Vec<u8>],
) -> Result<(), GitError> {
    let mut args: Vec<&OsStr> = vec![OsStr::new("-c"), OsStr::new("core.splitIndex=false")];
    args.extend(read_args(
        "add",
        &[
            "--intent-to-add",
            "--pathspec-from-file=-",
            "--pathspec-file-nul",
        ],
    ));
    let stdin: Vec<u8> = paths
        .iter()
        .flat_map(|path| path.iter().copied().chain([0]))
        .collect();
    git.run_cancellable(&args, top, Some(&stdin), env, cancel)?;
    Ok(())
}

/// Reads `status --porcelain=v2` of the index `env` names (the runner's own when it is empty).
fn porcelain(
    git: &Git,
    top: &Path,
    cancel: &Cancel,
    env: &[(&str, &str)],
) -> Result<Status, GitError> {
    let args = read_args(
        "status",
        &[
            "--porcelain=v2",
            "-z",
            "--branch",
            "--untracked-files=all",
            "--find-renames",
        ],
    );
    query(git, top, cancel, &args, None, env, parse::status)
}

/// Maps a status read with `untracked` added with intent to add back to what the user sees.
///
/// An added path that was untracked is [`Entry::Untracked`] again, in the spelling `ls-files`
/// gave (a nested repository's `/` included); when the index also changed it (a file removed
/// with `git rm --cached` is staged as deleted and untracked), that change stays as an
/// [`Entry::Changed`] of its own. The untracked entries come last, as in git's own status. A
/// rename whose new path was untracked is already an unstaged rename.
fn paired(status: Status, untracked: &[Vec<u8>]) -> Status {
    let spelling: HashMap<&[u8], &[u8]> = untracked
        .iter()
        .map(|path| (path.strip_suffix(b"/").unwrap_or(path), path.as_slice()))
        .collect();
    let mut entries = Vec::with_capacity(status.entries.len());
    let mut untracked_entries = Vec::new();
    for entry in status.entries {
        match entry {
            Entry::Changed {
                path,
                staged,
                unstaged: Change::Added,
            } => match spelling.get(path.as_slice()) {
                Some(spelled) => {
                    let untracked = spelled.to_vec();
                    if staged != Change::Unchanged {
                        entries.push(Entry::Changed {
                            path,
                            staged,
                            unstaged: Change::Unchanged,
                        });
                    }
                    untracked_entries.push(Entry::Untracked { path: untracked });
                }
                None => entries.push(Entry::Changed {
                    path,
                    staged,
                    unstaged: Change::Added,
                }),
            },
            untracked @ Entry::Untracked { .. } => untracked_entries.push(untracked),
            other => entries.push(other),
        }
    }
    entries.append(&mut untracked_entries);
    Status {
        branch: status.branch,
        entries,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    /// A repository in a temp folder, an isolated runner, and a fresh folder for the temporary
    /// index.
    struct Fixture {
        _dir: tempfile::TempDir,
        repo: PathBuf,
        temp: PathBuf,
        git: Git,
    }

    impl Fixture {
        fn new() -> Result<Fixture, Box<dyn std::error::Error>> {
            let dir = tempfile::tempdir()?;
            let config = dir.path().join("gitconfig");
            std::fs::write(&config, "")?;
            let git = Git::find()?.with_env(&[
                ("GIT_CONFIG_GLOBAL", config.to_str().ok_or("config path")?),
                ("GIT_CONFIG_NOSYSTEM", "1"),
            ]);
            let repo = dir.path().join("repo");
            let temp = dir.path().join("temp");
            std::fs::create_dir_all(&repo)?;
            std::fs::create_dir_all(&temp)?;
            git.run(
                &[OsStr::new("init"), OsStr::new("-q"), OsStr::new(".")],
                &repo,
                None,
                &[],
            )?;
            Ok(Fixture {
                _dir: dir,
                repo,
                temp,
                git,
            })
        }

        /// The names in the temp folder.
        fn temp_entries(&self) -> Result<Vec<PathBuf>, std::io::Error> {
            std::fs::read_dir(&self.temp)?
                .map(|entry| entry.map(|entry| entry.path()))
                .collect()
        }
    }

    fn untracked(path: &[u8]) -> Entry {
        Entry::Untracked {
            path: path.to_vec(),
        }
    }

    #[test]
    fn the_temporary_index_is_gone_after_a_success() -> TestResult {
        let fixture = Fixture::new()?;
        std::fs::write(fixture.repo.join("new.md"), "new\n")?;
        let status = status_in(&fixture.git, &fixture.repo, &Cancel::new(), &fixture.temp)?;
        assert_eq!(status.entries, vec![untracked(b"new.md")]);
        assert_eq!(fixture.temp_entries()?, Vec::<PathBuf>::new());
        Ok(())
    }

    #[test]
    fn the_temporary_index_is_gone_after_an_error() -> TestResult {
        let fixture = Fixture::new()?;
        let not_a_repo = fixture.temp.join("not-a-repo");
        std::fs::create_dir_all(&not_a_repo)?;
        let other = tempfile::tempdir()?;
        let result = status_in(&fixture.git, &not_a_repo, &Cancel::new(), other.path());
        assert!(matches!(result, Err(GitError::Failed { .. })), "{result:?}");
        assert_eq!(std::fs::read_dir(other.path())?.count(), 0);
        Ok(())
    }

    #[test]
    fn the_temporary_index_is_gone_after_a_cancel_before_the_start() -> TestResult {
        let fixture = Fixture::new()?;
        std::fs::write(fixture.repo.join("new.md"), "new\n")?;
        let cancel = Cancel::new();
        cancel.cancel();
        let result = status_in(&fixture.git, &fixture.repo, &cancel, &fixture.temp);
        assert!(
            matches!(result, Err(GitError::Cancelled { .. })),
            "{result:?}"
        );
        assert_eq!(fixture.temp_entries()?, Vec::<PathBuf>::new());
        Ok(())
    }

    #[test]
    fn the_temporary_index_is_gone_after_a_cancel_while_it_exists() -> TestResult {
        let fixture = Fixture::new()?;
        for number in 0..300 {
            std::fs::write(fixture.repo.join(format!("new-{number}.md")), "new\n")?;
        }
        let cancel = Cancel::new();
        let (result, seen) = std::thread::scope(|scope| {
            let watcher = scope.spawn(|| {
                // Cancels as soon as the temporary folder exists, so a later step is cancelled.
                let started = std::time::Instant::now();
                while started.elapsed() < std::time::Duration::from_secs(20) {
                    if std::fs::read_dir(&fixture.temp).is_ok_and(|mut dir| dir.next().is_some()) {
                        cancel.cancel();
                        return true;
                    }
                    std::thread::yield_now();
                }
                false
            });
            let result = status_in(&fixture.git, &fixture.repo, &cancel, &fixture.temp);
            (result, watcher.join())
        });
        assert_eq!(seen.ok(), Some(true), "the temporary folder was never seen");
        assert!(
            matches!(result, Err(GitError::Cancelled { .. })),
            "{result:?}"
        );
        assert_eq!(fixture.temp_entries()?, Vec::<PathBuf>::new());
        Ok(())
    }

    #[test]
    fn a_listed_file_gone_before_add_is_left_out() -> TestResult {
        let fixture = Fixture::new()?;
        std::fs::write(fixture.repo.join("new.md"), "new\n")?;
        // Listed by step 1, then removed (an editor's temporary file renamed over a record).
        let listed = vec![b"gone.md".to_vec(), b"new.md".to_vec()];
        let status = status_from(
            &fixture.git,
            &fixture.repo,
            &Cancel::new(),
            &fixture.temp,
            listed,
        )?;
        assert_eq!(status.entries, vec![untracked(b"new.md")]);
        assert_eq!(fixture.temp_entries()?, Vec::<PathBuf>::new());
        Ok(())
    }

    #[test]
    fn a_failed_add_is_repeated_once_with_a_fresh_list() -> TestResult {
        let fixture = Fixture::new()?;
        std::fs::write(fixture.repo.join(".gitignore"), "*.tmp\n")?;
        std::fs::write(fixture.repo.join("scratch.tmp"), "ignored\n")?;
        std::fs::write(fixture.repo.join("new.md"), "new\n")?;
        // `scratch.tmp` still exists but is ignored by now, so `add` refuses it.
        let listed = vec![b"scratch.tmp".to_vec(), b"new.md".to_vec()];
        let status = status_from(
            &fixture.git,
            &fixture.repo,
            &Cancel::new(),
            &fixture.temp,
            listed,
        )?;
        assert_eq!(
            status.entries,
            vec![untracked(b".gitignore"), untracked(b"new.md")]
        );
        assert_eq!(fixture.temp_entries()?, Vec::<PathBuf>::new());
        Ok(())
    }

    #[test]
    fn every_listed_file_gone_reads_the_real_index() -> TestResult {
        let fixture = Fixture::new()?;
        let status = status_from(
            &fixture.git,
            &fixture.repo,
            &Cancel::new(),
            &fixture.temp,
            vec![b"gone.md".to_vec()],
        )?;
        assert_eq!(status.entries, Vec::new());
        assert_eq!(fixture.temp_entries()?, Vec::<PathBuf>::new());
        Ok(())
    }

    fn branch() -> Branch {
        Branch {
            head: Some("main".to_owned()),
            commit: None,
            upstream: None,
        }
    }

    #[test]
    fn an_added_path_that_was_untracked_is_untracked_again_and_comes_last() {
        let status = Status {
            branch: branch(),
            entries: vec![
                Entry::Changed {
                    path: b"new.md".to_vec(),
                    staged: Change::Unchanged,
                    unstaged: Change::Added,
                },
                Entry::Changed {
                    path: b"mine.md".to_vec(),
                    staged: Change::Unchanged,
                    unstaged: Change::Added,
                },
                Entry::Changed {
                    path: b"removed.md".to_vec(),
                    staged: Change::Deleted,
                    unstaged: Change::Added,
                },
                Entry::Changed {
                    path: b"nested".to_vec(),
                    staged: Change::Unchanged,
                    unstaged: Change::Added,
                },
            ],
        };
        let untracked_paths = [
            b"new.md".to_vec(),
            b"removed.md".to_vec(),
            b"nested/".to_vec(),
        ];
        assert_eq!(
            paired(status, &untracked_paths).entries,
            vec![
                Entry::Changed {
                    path: b"mine.md".to_vec(),
                    staged: Change::Unchanged,
                    unstaged: Change::Added,
                },
                Entry::Changed {
                    path: b"removed.md".to_vec(),
                    staged: Change::Deleted,
                    unstaged: Change::Unchanged,
                },
                untracked(b"new.md"),
                untracked(b"removed.md"),
                untracked(b"nested/"),
            ]
        );
    }
}
