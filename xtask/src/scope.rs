//! Which checks a run needs: every one, or only `commit-msg` for a docs-only change
//! (`docs/specs/phases/P50-leaner-ci.md` §5.3).
//!
//! CI jobs and the local `cargo xtask gate` decide here, so they decide alike. Every uncertain
//! case adds checks; none removes one.

use std::path::Path;

use crate::Result;
use crate::changes::{self, Changes};
use crate::ci::CiEvent;
use crate::code_paths::CodePaths;
use crate::git::Git;

/// Why a skipped check is not needed, for the verdict table.
pub const NOT_NEEDED_REASON: &str = "the change touches only paths of the non-code list of \
gate/code-paths.toml, which no check reads (docs/gate.md §1)";

/// What a run has to check.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Scope {
    /// Every check runs; `note` says why when the reason is an uncertainty, not a code change.
    Every {
        /// Why the scope could not be narrowed, if that is the reason.
        note: Option<String>,
    },
    /// Only `commit-msg` runs: the change touches no path a check reads.
    DocsOnly,
}

fn every() -> Scope {
    Scope::Every { note: None }
}

fn every_because(note: impl Into<String>) -> Scope {
    Scope::Every {
        note: Some(note.into()),
    }
}

/// The scope of a change, by the rules of §5.3 in order: a `phase/` branch runs every check; an
/// unreadable change list runs every check; no changed path runs every check; only non-code paths
/// is docs-only; anything else runs every check.
#[must_use]
pub fn decide(changes: Result<Changes>, paths: &CodePaths) -> Scope {
    let changes = match changes {
        Ok(changes) => changes,
        Err(error) => {
            return every_because(format!("the changed paths could not be read: {error}"));
        }
    };
    if changes
        .branch
        .as_deref()
        .is_some_and(|branch| branch.starts_with("phase/"))
    {
        return every();
    }
    if changes.paths.is_empty() || !changes.paths.iter().all(|path| paths.is_non_code(path)) {
        return every();
    }
    Scope::DocsOnly
}

/// The scope of this run in the checkout at `root`: a push or any other event runs every check, a
/// pull request decides from its commits, and a run outside CI decides from the working tree.
#[must_use]
pub fn for_run(root: &Path, ci: Option<&CiEvent>) -> Scope {
    match ci {
        Some(CiEvent::Push(_) | CiEvent::Other) => every(),
        Some(CiEvent::PullRequest(pr)) => with_lists(root, |git, paths| {
            decide(changes::of_pull_request(git, pr), paths)
        }),
        None => with_lists(root, |git, paths| match changes::of_worktree(git) {
            Ok(Changes { branch: None, .. }) => {
                every_because("HEAD is detached, so there is no branch")
            }
            other => decide(other, paths),
        }),
    }
}

/// Runs `decide_with` over the non-code lists of `root`; lists that cannot be read run every check.
fn with_lists(root: &Path, decide_with: impl FnOnce(&Git, &CodePaths) -> Scope) -> Scope {
    match CodePaths::load(root) {
        Ok(paths) => decide_with(&Git::new(root), &paths),
        Err(error) => every_because(format!("the non-code list could not be read: {error}")),
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::Path;

    use super::*;
    use crate::ci::{PullRequestEvent, PushEvent};
    use crate::git::testing::{TestResult, commit, git, init_repo};

    const REAL_LISTS: &str = include_str!("../../gate/code-paths.toml");

    fn lists() -> CodePaths {
        CodePaths::parse(REAL_LISTS).expect("the real lists parse")
    }

    fn changes(branch: &str, paths: &[&str]) -> Changes {
        Changes {
            branch: Some(branch.to_owned()),
            paths: paths.iter().map(|path| (*path).to_owned()).collect(),
        }
    }

    fn write(dir: &Path, path: &str, text: &str) -> TestResult {
        let full = dir.join(path);
        if let Some(parent) = full.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(full, text)?;
        Ok(())
    }

    // AC: CE-08C.2
    #[test]
    fn docs_only_changes_on_a_docs_branch_need_only_commit_msg() {
        let scope = decide(Ok(changes("docs/x", &["docs/a.md", "README.md"])), &lists());
        assert_eq!(scope, Scope::DocsOnly);
    }

    // AC: CE-08C.2
    #[test]
    fn a_phase_branch_runs_every_check() {
        let scope = decide(Ok(changes("phase/P50-leaner-ci", &["docs/x.md"])), &lists());
        assert_eq!(scope, every());
    }

    // AC: CE-08C.2
    #[test]
    fn a_malformed_phase_branch_runs_every_check() {
        let scope = decide(Ok(changes("phase/foo", &["docs/x.md"])), &lists());
        assert_eq!(scope, every());
    }

    // AC: CE-08C.2
    #[test]
    fn a_code_path_runs_every_check() {
        let scope = decide(
            Ok(changes("docs/x", &["docs/a.md", "xtask/src/x.rs"])),
            &lists(),
        );
        assert_eq!(scope, every());
    }

    // AC: CE-08C.2
    #[test]
    fn an_unreadable_change_list_runs_every_check() {
        let error = crate::Error::Config {
            file: "somewhere".to_owned(),
            message: "boom-text".to_owned(),
        };
        match decide(Err(error), &lists()) {
            Scope::Every { note: Some(note) } => assert!(note.contains("boom-text"), "{note}"),
            other => panic!("expected Every with a note, got {other:?}"),
        }
    }

    // AC: CE-08C.2
    #[test]
    fn no_change_runs_every_check() {
        assert_eq!(decide(Ok(changes("docs/x", &[])), &lists()), every());
    }

    fn docs_repo() -> TestResult<tempfile::TempDir> {
        let dir = init_repo()?;
        let path = dir.path();
        write(path, "gate/code-paths.toml", REAL_LISTS)?;
        write(path, "README.md", "x\n")?;
        let initial = commit(path, "chore: initial")?;
        git(path, &["update-ref", "refs/remotes/origin/main", &initial])?;
        Ok(dir)
    }

    // AC: CE-08C.2
    #[test]
    fn a_push_runs_every_check() -> TestResult {
        let dir = docs_repo()?;
        let event = CiEvent::Push(PushEvent {
            before: "a".to_owned(),
            after: "b".to_owned(),
        });
        assert_eq!(for_run(dir.path(), Some(&event)), every());
        Ok(())
    }

    // AC: CE-08C.2
    #[test]
    fn uncommitted_code_on_a_docs_branch_runs_every_check() -> TestResult {
        let dir = docs_repo()?;
        let path = dir.path();
        git(path, &["switch", "-q", "-c", "docs/x"])?;
        write(path, "docs/a.md", "a\n")?;
        commit(path, "docs: a")?;
        assert_eq!(for_run(path, None), Scope::DocsOnly);
        write(path, "crates/a/src/b.rs", "// b\n")?;
        assert_eq!(for_run(path, None), every());
        Ok(())
    }

    #[test]
    fn a_detached_head_runs_every_check_with_a_note() -> TestResult {
        let dir = docs_repo()?;
        git(dir.path(), &["switch", "-q", "--detach"])?;
        assert!(matches!(
            for_run(dir.path(), None),
            Scope::Every { note: Some(_) }
        ));
        Ok(())
    }

    #[test]
    fn missing_lists_run_every_check_with_a_note() -> TestResult {
        let dir = init_repo()?;
        commit(dir.path(), "chore: initial")?;
        assert!(matches!(
            for_run(dir.path(), None),
            Scope::Every { note: Some(_) }
        ));
        Ok(())
    }

    #[test]
    fn a_pull_request_event_decides_from_its_commits() -> TestResult {
        let dir = docs_repo()?;
        let path = dir.path();
        let base = git(path, &["rev-parse", "HEAD"])?;
        git(path, &["switch", "-q", "-c", "docs/x"])?;
        write(path, "docs/a.md", "a\n")?;
        let head = commit(path, "docs: a")?;
        let event = |head_ref: &str| {
            CiEvent::PullRequest(PullRequestEvent {
                action: "opened".to_owned(),
                number: 1,
                title: "docs: a".to_owned(),
                body: None,
                base_sha: base.clone(),
                head_sha: head.clone(),
                head_ref: head_ref.to_owned(),
                label: None,
            })
        };
        assert_eq!(for_run(path, Some(&event("docs/x"))), Scope::DocsOnly);
        assert_eq!(for_run(path, Some(&event("phase/P50-x"))), every());
        Ok(())
    }
}
