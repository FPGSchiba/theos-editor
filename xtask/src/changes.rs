//! The paths a change touches, read in one place: a pull request, or the working tree of a
//! developer's machine (`docs/gate.md`).
//!
//! `approval`, `slow-tests` and the scope decision of `cargo xtask gate` share this one diff
//! reader (renames, sorting). The scope measures from `origin/main`; `approval` and `slow-tests`
//! measure from the event's base (spec §5.3). A rename counts as the old path and the new
//! path ([`Git::diff_name_only_no_renames`]), so moving a file out of a folder still touches that
//! folder. Paths are repo-relative, `/`-separated (as git writes them), sorted and unique.

use crate::Result;
use crate::ci::PullRequestEvent;
use crate::git::Git;

/// What a change touches.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Changes {
    /// The branch the change is on; `None` for a detached `HEAD`.
    pub branch: Option<String>,
    /// The changed paths, sorted and unique.
    pub paths: Vec<String>,
}

/// The paths that differ between `merge_base` and `head`, sorted and unique.
///
/// # Errors
///
/// Fails when either revision is unknown or git cannot be run.
pub fn paths_between(git: &Git, merge_base: &str, head: &str) -> Result<Vec<String>> {
    Ok(sorted(git.diff_name_only_no_renames(merge_base, head)?))
}

/// What `pr` changes since the merge base of its base and head commits.
///
/// # Errors
///
/// Fails when either commit is not in the repository or git cannot be run.
pub fn of_pull_request(git: &Git, pr: &PullRequestEvent) -> Result<Changes> {
    let merge_base = git.merge_base(&pr.base_sha, &pr.head_sha)?;
    Ok(Changes {
        branch: Some(pr.head_ref.clone()),
        paths: paths_between(git, &merge_base, &pr.head_sha)?,
    })
}

/// What `pr` changes since the merge base of `origin/main` and its head commit: the event's base
/// is not read, because it can change later without a new run.
///
/// # Errors
///
/// Fails when `origin/main` or the head commit is not in the repository or git cannot be run.
pub fn of_pull_request_against_main(git: &Git, pr: &PullRequestEvent) -> Result<Changes> {
    let merge_base = git.merge_base("origin/main", &pr.head_sha)?;
    Ok(Changes {
        branch: Some(pr.head_ref.clone()),
        paths: paths_between(git, &merge_base, &pr.head_sha)?,
    })
}

/// What the working tree changes since the merge base of `HEAD` and `origin/main`: committed,
/// staged, unstaged and untracked files. Files git ignores are not changes.
///
/// # Errors
///
/// Fails when `origin/main` is unknown or git cannot be run.
pub fn of_worktree(git: &Git) -> Result<Changes> {
    let merge_base = git.merge_base("HEAD", "origin/main")?;
    let mut paths = git.diff_name_only_worktree(&merge_base)?;
    paths.extend(git.untracked()?);
    Ok(Changes {
        branch: git.current_branch()?,
        paths: sorted(paths),
    })
}

fn sorted(mut paths: Vec<String>) -> Vec<String> {
    paths.sort();
    paths.dedup();
    paths
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;
    use crate::git::testing::{TestResult, commit, git, init_repo};

    fn write(dir: &std::path::Path, path: &str, text: &str) -> TestResult {
        let full = dir.join(path);
        if let Some(parent) = full.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(full, text)?;
        Ok(())
    }

    fn pull_request(base: &str, head: &str) -> PullRequestEvent {
        PullRequestEvent {
            action: "opened".to_owned(),
            number: 1,
            title: "feat: x".to_owned(),
            body: None,
            base_sha: base.to_owned(),
            head_sha: head.to_owned(),
            head_ref: "phase/P50-x".to_owned(),
            label: None,
        }
    }

    // AC: CE-08C.2
    #[test]
    fn a_pull_request_lists_the_paths_since_the_merge_base() -> TestResult {
        let dir = init_repo()?;
        let path = dir.path();
        write(path, "README.md", "x\n")?;
        let fork = commit(path, "chore: initial")?;
        git(path, &["switch", "-q", "-c", "phase/P50-x"])?;
        write(path, "crates/a/src/lib.rs", "a\n")?;
        write(path, "docs/b.md", "b\n")?;
        let head = commit(path, "feat: branch")?;
        git(path, &["switch", "-q", "main"])?;
        write(path, "main-only.txt", "m\n")?;
        let base = commit(path, "feat: main moves on")?;
        assert_ne!(base, fork);

        let changes = of_pull_request(&Git::new(path), &pull_request(&base, &head))?;
        assert_eq!(
            changes,
            Changes {
                branch: Some("phase/P50-x".to_owned()),
                paths: vec!["crates/a/src/lib.rs".to_owned(), "docs/b.md".to_owned()],
            }
        );
        Ok(())
    }

    // AC: CE-08C.2
    #[test]
    fn a_rename_out_of_code_counts_both_paths() -> TestResult {
        let dir = init_repo()?;
        let path = dir.path();
        write(path, "crates/a/src/x.rs", "pub fn x() {}\n// padding\n")?;
        write(path, "docs/keep.md", "kept\n")?;
        let base = commit(path, "chore: initial")?;
        git(path, &["mv", "crates/a/src/x.rs", "docs/x.rs"])?;
        let head = commit(path, "refactor: move")?;

        let paths = paths_between(&Git::new(path), &base, &head)?;
        assert_eq!(paths, ["crates/a/src/x.rs", "docs/x.rs"]);
        Ok(())
    }

    #[test]
    fn the_worktree_counts_uncommitted_and_untracked_files() -> TestResult {
        let dir = init_repo()?;
        let path = dir.path();
        write(path, ".gitignore", "target/\n")?;
        write(path, "crates/b/src/lib.rs", "// one\n")?;
        let initial = commit(path, "chore: initial")?;
        git(path, &["update-ref", "refs/remotes/origin/main", &initial])?;
        git(path, &["switch", "-q", "-c", "phase/P50-x"])?;
        write(path, "docs/a.md", "a\n")?;
        commit(path, "docs: a")?;
        write(path, "crates/b/src/lib.rs", "// two\n")?;
        write(path, "crates/c/src/new.rs", "// new\n")?;
        write(path, "target/debug/out.txt", "ignored\n")?;

        let changes = of_worktree(&Git::new(path))?;
        assert_eq!(changes.branch.as_deref(), Some("phase/P50-x"));
        assert_eq!(
            changes.paths,
            ["crates/b/src/lib.rs", "crates/c/src/new.rs", "docs/a.md"]
        );
        Ok(())
    }

    #[test]
    fn the_worktree_needs_origin_main() -> TestResult {
        let dir = init_repo()?;
        commit(dir.path(), "chore: initial")?;
        let result = of_worktree(&Git::new(dir.path()));
        assert!(result.is_err(), "{result:?}");
        Ok(())
    }

    #[test]
    fn paths_use_forward_slashes() -> TestResult {
        let dir = init_repo()?;
        let path = dir.path();
        let base = commit(path, "chore: initial")?;
        write(path, "a/b/c/d.txt", "d\n")?;
        let head = commit(path, "feat: deep")?;
        let paths = paths_between(&Git::new(path), &base, &head)?;
        assert_eq!(paths, ["a/b/c/d.txt"]);
        assert!(paths.iter().all(|path| !path.contains('\\')));
        Ok(())
    }
}
