//! The coverage of the commit a pull request is compared with.
//!
//! [`measure`] gives the per-crate coverage of a commit, which is the merge base of the pull
//! request and `main`. The commit is checked out in a temporary work tree at
//! `target/xtask/coverage/wt`, the tests run there under `cargo llvm-cov` with their own build
//! directory (`target/xtask/coverage/target`, so that the head's build is left alone), and the
//! work tree is removed again whatever happened. A work tree left behind by a run that was killed
//! is replaced.
//!
//! The result is cached in `target/xtask/coverage/cache/<sha>-<toolchain>-<llvm-cov>.json`, with
//! the commit, the first twelve hex digits of the sha256 of the commit's `rust-toolchain.toml`
//! (`none` when it has none) and the version of cargo-llvm-cov. The file holds the per-crate
//! numbers, not cargo-llvm-cov's export, because an export has the absolute paths of the place it
//! was made in and the numbers are all the comparison needs. An entry that cannot be read is
//! measured again.
//!
//! A commit without `xtask/Cargo.toml` has no baseline: it is the one before the gate existed,
//! and there is nothing to run there, so [`measure`] gives `None`.

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

use super::{CargoLlvm, CrateCoverage, Llvm, io_error, llvm_cov_command, parse_llvm_cov, work_dir};
use crate::git::Git;
use crate::{Error, Result};

type Coverage = BTreeMap<String, CrateCoverage>;

/// The file whose presence says that the commit has the gate.
const XTASK_MANIFEST: &str = "xtask/Cargo.toml";

/// The coverage of the commit `sha` per crate, from the cache in `cache_dir` or measured in a
/// temporary work tree, and `None` when the commit has no `xtask/Cargo.toml`.
///
/// # Errors
///
/// Fails when `sha` is unknown, git or cargo cannot run, the tests fail at the commit, or the
/// export is not what cargo-llvm-cov writes.
pub fn measure(root: &Path, sha: &str, cache_dir: &Path) -> Result<Option<Coverage>> {
    measure_with(&CargoLlvm, root, sha, cache_dir)
}

pub(crate) fn measure_with(
    llvm: &dyn Llvm,
    root: &Path,
    sha: &str,
    cache_dir: &Path,
) -> Result<Option<Coverage>> {
    let git = Git::new(root);
    let sha = git.rev_parse(sha)?;
    if git.show(&sha, XTASK_MANIFEST)?.is_none() {
        return Ok(None);
    }
    let cache = cache_file(llvm, &git, root, &sha, cache_dir)?;
    if let Some(cached) = read_cache(&cache) {
        return Ok(Some(cached));
    }
    let work = work_dir(root);
    let measured = in_worktree(&git, &work.join("wt"), &sha, |tree| {
        let export = work.join("base.json");
        // An export of an earlier run must not be taken for this one.
        if export.exists() {
            fs::remove_file(&export).map_err(io_error(&export))?;
        }
        let target_dir = work.join("target");
        if !llvm.run(tree, &export, Some(&target_dir))? {
            return Err(Error::Command {
                program: "cargo".to_owned(),
                args: llvm_cov_command(&export, Some(&target_dir)).args,
                status: None,
                stderr: format!(
                    "the tests failed at the baseline commit {}, so its coverage cannot be \
                     measured; cargo's output is above",
                    sha.get(..12).unwrap_or(&sha)
                ),
            });
        }
        let json = fs::read_to_string(&export).map_err(io_error(&export))?;
        parse_llvm_cov(&json, tree)
    })?;
    write_cache(&cache, &measured)?;
    Ok(Some(measured))
}

/// Puts `coverage`, the head result of the commit `sha`, in the cache where [`measure`] looks
/// for a baseline.
pub(crate) fn store(
    llvm: &dyn Llvm,
    root: &Path,
    sha: &str,
    cache_dir: &Path,
    coverage: &Coverage,
) -> Result<()> {
    let git = Git::new(root);
    let sha = git.rev_parse(sha)?;
    let cache = cache_file(llvm, &git, root, &sha, cache_dir)?;
    write_cache(&cache, coverage)
}

/// `<cache_dir>/<sha>-<toolchain>-<llvm-cov>.json`, see the module documentation.
fn cache_file(
    llvm: &dyn Llvm,
    git: &Git,
    root: &Path,
    sha: &str,
    cache_dir: &Path,
) -> Result<PathBuf> {
    let toolchain = git
        .show(sha, "rust-toolchain.toml")?
        .map_or_else(|| "none".to_owned(), |text| short_hash(&text));
    let version = llvm.version(root)?;
    Ok(cache_dir.join(format!("{sha}-{toolchain}-{version}.json")))
}

/// The first twelve hex digits of the sha256 of `text`.
fn short_hash(text: &str) -> String {
    Sha256::digest(text.as_bytes())
        .iter()
        .take(6)
        .fold(String::new(), |mut hex, byte| {
            // Writing to a `String` cannot fail.
            let _ = write!(hex, "{byte:02x}");
            hex
        })
}

fn read_cache(path: &Path) -> Option<Coverage> {
    let text = fs::read_to_string(path).ok()?;
    let coverage: Coverage = serde_json::from_str(&text).ok()?;
    coverage
        .values()
        .all(|crate_| crate_.covered <= crate_.count)
        .then_some(coverage)
}

/// Writes the entry through a temporary file, so that a run that is killed does not leave half
/// of one.
fn write_cache(path: &Path, coverage: &Coverage) -> Result<()> {
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir).map_err(io_error(dir))?;
    }
    let mut text = serde_json::to_string_pretty(coverage).map_err(|_| Error::Parse {
        what: "the coverage to cache".to_owned(),
        message: "it cannot be written as JSON".to_owned(),
    })?;
    text.push('\n');
    let temporary = path.with_extension("json.tmp");
    fs::write(&temporary, text).map_err(io_error(&temporary))?;
    fs::rename(&temporary, path).map_err(io_error(path))
}

/// Runs `work` in a new work tree of `sha` at `dir`, and removes the work tree afterwards, also
/// when `work` fails. The error of `work` comes before one of the removal.
fn in_worktree<T>(
    git: &Git,
    dir: &Path,
    sha: &str,
    work: impl FnOnce(&Path) -> Result<T>,
) -> Result<T> {
    add_worktree(git, dir, sha)?;
    let result = work(dir);
    let removed = remove_worktree(git, dir);
    let value = result?;
    removed?;
    Ok(value)
}

/// Checks `sha` out at `dir`, after clearing what an earlier run that was killed left there.
fn add_worktree(git: &Git, dir: &Path, sha: &str) -> Result<()> {
    if dir.exists() {
        clear(git, dir)?;
    }
    if git.worktree_add_detached(dir, sha).is_ok() {
        return Ok(());
    }
    // A work tree that git still knows although its directory is gone.
    clear(git, dir)?;
    git.worktree_prune()?;
    git.worktree_add_detached(dir, sha)
}

/// Removes the work tree at `dir` and then the directory, if either is still there.
fn clear(git: &Git, dir: &Path) -> Result<()> {
    // It may be no work tree at all, only a directory.
    let _ = git.worktree_remove(dir);
    if dir.exists() {
        fs::remove_dir_all(dir).map_err(io_error(dir))?;
    }
    Ok(())
}

/// Removes the work tree at `dir` so that neither it nor git's record of it is left.
fn remove_worktree(git: &Git, dir: &Path) -> Result<()> {
    let removed = git.worktree_remove(dir);
    if removed.is_ok() && !dir.exists() {
        return Ok(());
    }
    // Git could not take it away, such as a file that something still holds open.
    clear(git, dir)?;
    git.worktree_prune()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::git::testing::{TestResult, commit, git, init_repo};
    use serde_json::json;
    use std::cell::{Cell, RefCell};

    const TOOLCHAIN: &str = "[toolchain]\nchannel = \"1.99.1\"\n";
    /// The first twelve hex digits of the sha256 of `TOOLCHAIN`.
    const TOOLCHAIN_HASH: &str = "5a7b7b624394";

    /// A `cargo llvm-cov` that gives `covered` of 100 lines to crate `a`, and notes what it saw
    /// in the directory it ran in.
    struct FakeLlvm {
        version: &'static str,
        covered: u64,
        fails: bool,
        runs: Cell<u32>,
        markers: RefCell<Vec<String>>,
    }

    impl FakeLlvm {
        fn new(covered: u64) -> Self {
            Self {
                version: "0.9.1",
                covered,
                fails: false,
                runs: Cell::new(0),
                markers: RefCell::new(Vec::new()),
            }
        }
    }

    impl Llvm for FakeLlvm {
        fn version(&self, _root: &Path) -> Result<String> {
            Ok(self.version.to_owned())
        }

        fn run(&self, dir: &Path, output: &Path, target_dir: Option<&Path>) -> Result<bool> {
            self.runs.set(self.runs.get() + 1);
            assert!(target_dir.is_some_and(|target| target.ends_with("target")));
            let marker = fs::read_to_string(dir.join("marker.txt")).map_err(io_error(dir))?;
            self.markers.borrow_mut().push(marker);
            if self.fails {
                return Ok(false);
            }
            let export = json!({
                "type": "llvm.coverage.json.export",
                "data": [{ "files": [{
                    "filename": format!("{}/crates/a/src/lib.rs", dir.display()),
                    "summary": { "lines": { "count": 100, "covered": self.covered } },
                }]}],
            });
            fs::write(output, export.to_string()).map_err(io_error(output))?;
            Ok(true)
        }
    }

    /// Leaves something at the path of the work tree: the repository, the path, the commit.
    type Leave = fn(&Path, &Path, &str) -> TestResult;

    fn a(covered: u64) -> Coverage {
        BTreeMap::from([(
            "a".to_owned(),
            CrateCoverage {
                covered,
                count: 100,
            },
        )])
    }

    /// A repository whose first commit has `xtask/Cargo.toml`, the toolchain and a marker file,
    /// and whose second commit changes the marker. Returns the repository, its first commit
    /// and the cache directory.
    fn repo() -> TestResult<(tempfile::TempDir, String, PathBuf)> {
        let dir = init_repo()?;
        let path = dir.path();
        fs::write(path.join(".gitignore"), "target\n")?;
        fs::write(path.join("rust-toolchain.toml"), TOOLCHAIN)?;
        fs::create_dir(path.join("xtask"))?;
        fs::write(path.join("xtask").join("Cargo.toml"), "[package]\n")?;
        fs::write(path.join("marker.txt"), "first")?;
        let first = commit(path, "feat: first")?;
        fs::write(path.join("marker.txt"), "second")?;
        commit(path, "feat: second")?;
        let cache = work_dir(path).join("cache");
        Ok((dir, first, cache))
    }

    fn entries(cache: &Path) -> TestResult<Vec<String>> {
        let mut names = Vec::new();
        if cache.exists() {
            for entry in fs::read_dir(cache)? {
                names.push(entry?.file_name().to_string_lossy().into_owned());
            }
        }
        names.sort();
        Ok(names)
    }

    fn worktrees(dir: &Path) -> TestResult<usize> {
        Ok(git(dir, &["worktree", "list"])?.lines().count())
    }

    #[test]
    fn a_commit_without_xtask_has_no_baseline() -> TestResult {
        let dir = init_repo()?;
        fs::write(dir.path().join("marker.txt"), "x")?;
        let sha = commit(dir.path(), "feat: before the gate")?;
        let llvm = FakeLlvm::new(50);
        let cache = work_dir(dir.path()).join("cache");
        assert_eq!(measure_with(&llvm, dir.path(), &sha, &cache)?, None);
        assert_eq!(llvm.runs.get(), 0);
        assert_eq!(entries(&cache)?, Vec::<String>::new());
        Ok(())
    }

    #[test]
    fn the_baseline_is_measured_in_a_work_tree_of_its_commit_which_is_removed() -> TestResult {
        let (dir, first, cache) = repo()?;
        let llvm = FakeLlvm::new(70);
        let measured = measure_with(&llvm, dir.path(), &first, &cache)?;
        // The numbers are those of the export that was made inside the work tree.
        assert_eq!(measured, Some(a(70)));
        // The work tree had the commit that was asked for, not `HEAD`.
        assert_eq!(*llvm.markers.borrow(), ["first"]);
        assert_eq!(worktrees(dir.path())?, 1);
        assert!(!work_dir(dir.path()).join("wt").exists());
        Ok(())
    }

    #[test]
    fn the_cache_name_has_the_commit_the_toolchain_and_the_llvm_cov_version() -> TestResult {
        let (dir, first, cache) = repo()?;
        measure_with(&FakeLlvm::new(70), dir.path(), &first, &cache)?;
        assert_eq!(
            entries(&cache)?,
            [format!("{first}-{TOOLCHAIN_HASH}-0.9.1.json")]
        );
        // A short name for the commit gives the same entry.
        let llvm = FakeLlvm::new(10);
        assert_eq!(
            measure_with(&llvm, dir.path(), &first[..10], &cache)?,
            Some(a(70))
        );
        assert_eq!(llvm.runs.get(), 0);
        Ok(())
    }

    #[test]
    fn a_commit_without_a_toolchain_file_has_its_own_name() -> TestResult {
        let (dir, _, cache) = repo()?;
        git(dir.path(), &["rm", "-q", "rust-toolchain.toml"])?;
        let sha = commit(dir.path(), "chore: drop the toolchain file")?;
        measure_with(&FakeLlvm::new(70), dir.path(), &sha, &cache)?;
        assert_eq!(entries(&cache)?, [format!("{sha}-none-0.9.1.json")]);
        Ok(())
    }

    #[test]
    fn a_cached_baseline_is_not_measured_again() -> TestResult {
        let (dir, first, cache) = repo()?;
        let llvm = FakeLlvm::new(70);
        measure_with(&llvm, dir.path(), &first, &cache)?;
        let again = FakeLlvm::new(10);
        assert_eq!(
            measure_with(&again, dir.path(), &first, &cache)?,
            Some(a(70))
        );
        assert_eq!(again.runs.get(), 0);
        Ok(())
    }

    #[test]
    fn a_cache_entry_of_another_toolchain_or_llvm_cov_is_not_used() -> TestResult {
        let (dir, first, cache) = repo()?;
        fs::create_dir_all(&cache)?;
        let stale = serde_json::to_string(&a(1))?;
        fs::write(
            cache.join(format!("{first}-000000000000-0.9.1.json")),
            &stale,
        )?;
        fs::write(
            cache.join(format!("{first}-{TOOLCHAIN_HASH}-0.9.0.json")),
            &stale,
        )?;
        let llvm = FakeLlvm::new(70);
        assert_eq!(
            measure_with(&llvm, dir.path(), &first, &cache)?,
            Some(a(70))
        );
        assert_eq!(llvm.runs.get(), 1);
        assert_eq!(entries(&cache)?.len(), 3);
        // Another version of cargo-llvm-cov is another entry too.
        let newer = FakeLlvm {
            version: "0.9.2",
            ..FakeLlvm::new(60)
        };
        assert_eq!(
            measure_with(&newer, dir.path(), &first, &cache)?,
            Some(a(60))
        );
        assert_eq!(entries(&cache)?.len(), 4);
        Ok(())
    }

    #[test]
    fn a_toolchain_that_changed_with_the_commit_is_another_entry() -> TestResult {
        let (dir, first, cache) = repo()?;
        fs::write(
            dir.path().join("rust-toolchain.toml"),
            "[toolchain]\nchannel = \"2.0.0\"\n",
        )?;
        let second = commit(dir.path(), "chore: bump the toolchain")?;
        let llvm = FakeLlvm::new(70);
        measure_with(&llvm, dir.path(), &first, &cache)?;
        measure_with(&llvm, dir.path(), &second, &cache)?;
        let names = entries(&cache)?;
        assert_eq!(names.len(), 2, "{names:?}");
        let hashes: Vec<&str> = names
            .iter()
            .filter_map(|name| name.split('-').nth(1))
            .collect();
        assert_eq!(hashes.len(), 2);
        assert_ne!(hashes[0], hashes[1]);
        Ok(())
    }

    #[test]
    fn a_cache_entry_that_cannot_be_read_is_measured_again() -> TestResult {
        let (dir, first, cache) = repo()?;
        fs::create_dir_all(&cache)?;
        let entry = cache.join(format!("{first}-{TOOLCHAIN_HASH}-0.9.1.json"));
        let wrong = [
            "",
            "not json",
            "[1, 2]",
            r#"{"a": {"covered": "x", "count": 1}}"#,
            r#"{"a": {"covered": 5, "count": 1}}"#,
        ];
        for text in wrong {
            fs::write(&entry, text)?;
            let llvm = FakeLlvm::new(70);
            assert_eq!(
                measure_with(&llvm, dir.path(), &first, &cache)?,
                Some(a(70)),
                "{text}"
            );
            assert_eq!(llvm.runs.get(), 1, "{text}");
            assert_eq!(read_cache(&entry), Some(a(70)), "{text}");
        }
        Ok(())
    }

    #[test]
    fn a_failed_run_is_an_error_without_a_work_tree_or_a_cache_entry() -> TestResult {
        let (dir, first, cache) = repo()?;
        let llvm = FakeLlvm {
            fails: true,
            ..FakeLlvm::new(70)
        };
        let result = measure_with(&llvm, dir.path(), &first, &cache);
        let Err(Error::Command {
            program, stderr, ..
        }) = &result
        else {
            panic!("{result:?}");
        };
        assert_eq!(program, "cargo");
        assert!(stderr.contains(&first[..12]), "{stderr}");
        assert_eq!(worktrees(dir.path())?, 1);
        assert!(!work_dir(dir.path()).join("wt").exists());
        assert_eq!(entries(&cache)?, Vec::<String>::new());
        Ok(())
    }

    #[test]
    fn an_export_of_an_earlier_run_is_not_taken_for_this_one() -> TestResult {
        struct Silent;
        impl Llvm for Silent {
            fn version(&self, _root: &Path) -> Result<String> {
                Ok("0.9.1".to_owned())
            }
            fn run(&self, _dir: &Path, _out: &Path, _target: Option<&Path>) -> Result<bool> {
                Ok(true)
            }
        }
        let (dir, first, cache) = repo()?;
        let work = work_dir(dir.path());
        fs::create_dir_all(&work)?;
        fs::write(work.join("base.json"), r#"{"data": []}"#)?;
        let result = measure_with(&Silent, dir.path(), &first, &cache);
        assert!(matches!(result, Err(Error::Io { .. })), "{result:?}");
        assert_eq!(worktrees(dir.path())?, 1);
        Ok(())
    }

    #[test]
    fn an_unknown_commit_is_an_error() -> TestResult {
        let (dir, _, cache) = repo()?;
        let llvm = FakeLlvm::new(70);
        let unknown = measure_with(&llvm, dir.path(), "no-such-commit", &cache);
        assert!(matches!(unknown, Err(Error::Command { .. })), "{unknown:?}");
        let option = measure_with(&llvm, dir.path(), "--output=x", &cache);
        assert!(matches!(option, Err(Error::Parse { .. })), "{option:?}");
        assert_eq!(llvm.runs.get(), 0);
        Ok(())
    }

    #[test]
    fn what_a_killed_run_left_behind_is_replaced() -> TestResult {
        let (dir, first, cache) = repo()?;
        let path = dir.path();
        let tree = work_dir(path).join("wt");
        let leftovers: [(&str, Leave); 3] = [
            ("a registered work tree", |path, tree, first| {
                git(
                    path,
                    &[
                        "worktree",
                        "add",
                        "-q",
                        "--detach",
                        &tree.to_string_lossy(),
                        first,
                    ],
                )?;
                Ok(())
            }),
            ("a directory that is no work tree", |_, tree, _| {
                fs::create_dir_all(tree.join("deep"))?;
                fs::write(tree.join("deep").join("file.txt"), "left")?;
                Ok(())
            }),
            (
                "a registered work tree whose directory is gone",
                |path, tree, first| {
                    git(
                        path,
                        &[
                            "worktree",
                            "add",
                            "-q",
                            "--detach",
                            &tree.to_string_lossy(),
                            first,
                        ],
                    )?;
                    fs::remove_dir_all(tree)?;
                    Ok(())
                },
            ),
        ];
        for (what, leave) in leftovers {
            leave(path, &tree, &first)?;
            let llvm = FakeLlvm::new(70);
            let measured = measure_with(&llvm, path, &first, &cache)?;
            assert_eq!(measured, Some(a(70)), "{what}");
            assert_eq!(worktrees(path)?, 1, "{what}");
            assert!(!tree.exists(), "{what}");
            fs::remove_dir_all(&cache)?;
        }
        Ok(())
    }

    #[test]
    fn the_head_result_of_a_push_is_what_the_next_pull_request_finds() -> TestResult {
        let (dir, first, cache) = repo()?;
        store(&FakeLlvm::new(0), dir.path(), &first, &cache, &a(88))?;
        assert_eq!(
            entries(&cache)?,
            [format!("{first}-{TOOLCHAIN_HASH}-0.9.1.json")]
        );
        let llvm = FakeLlvm::new(10);
        assert_eq!(
            measure_with(&llvm, dir.path(), &first, &cache)?,
            Some(a(88))
        );
        assert_eq!(llvm.runs.get(), 0);
        // The entry is written whole, without the temporary file it went through.
        assert_eq!(entries(&cache)?.len(), 1);
        Ok(())
    }

    #[test]
    fn the_short_hash_is_twelve_hex_digits_of_sha256() {
        assert_eq!(short_hash(TOOLCHAIN), TOOLCHAIN_HASH);
        assert_eq!(short_hash(""), "e3b0c44298fc");
    }
}
