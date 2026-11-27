//! Which checks a run needs: every one, or only `commit-msg` for a docs-only change
//! (`docs/specs/phases/P50-leaner-ci.md` §5.3).
//!
//! CI jobs and the local `cargo xtask gate` decide here, so they decide alike. Every uncertain
//! case adds checks; none removes one. A run that a label started is a deliberate request
//! (§5.6) and runs every check.

use std::path::Path;

use crate::Result;
use crate::changes::{self, Changes};
use crate::ci::CiEvent;
use crate::code_paths::CodePaths;
use crate::git::Git;
use crate::verdict::Check;

/// Why a skipped check is not needed, for the verdict table.
pub const NOT_NEEDED_REASON: &str = "the change touches only paths of the non-code list of \
gate/code-paths.toml, which no check reads (docs/gate.md §1)";

/// What a run has to check.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Scope {
    /// Every check runs; `note` says why when the reason is not a change to code.
    Every {
        /// Why every check runs, when it is not because the change touches code.
        note: Option<String>,
    },
    /// Only `commit-msg` runs: the change touches no path a check reads.
    DocsOnly,
}

/// Whether `check` runs under `scope`: every check does, but for a change that touches no code
/// only `commit-msg`. `gate` and `needs` both ask here, so they cannot disagree.
#[must_use]
pub fn runs(scope: &Scope, check: Check) -> bool {
    *scope != Scope::DocsOnly || check == Check::CommitMsg
}

fn every() -> Scope {
    Scope::Every { note: None }
}

fn every_because(note: impl Into<String>) -> Scope {
    Scope::Every {
        note: Some(note.into()),
    }
}

/// The scope of a change, by the rules of §5.3 in order: an unreadable change list, a `phase/`
/// branch and no changed path each run every check, with a note; only non-code paths is
/// docs-only; anything else (code) runs every check without a note.
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
        return every_because(
            "the branch is a phase branch, and `ac-trace` reads the phase from it",
        );
    }
    if changes.paths.is_empty() {
        return every_because("no changed path was found");
    }
    if !changes.paths.iter().all(|path| paths.is_non_code(path)) {
        return every();
    }
    Scope::DocsOnly
}

/// Why a run that a label started runs every check.
const LABEL_NOTE: &str =
    "a label started this run, a deliberate request: every selected check runs";

/// The scope of this run in the checkout at `root`: a push or any other event runs every check, a
/// pull request decides from its commits since `origin/main` (not its base, which can change
/// without a new run), and a run outside CI decides from the working tree.
/// Every run of every check for a reason other than a code change carries a note that says why.
///
/// A pull request run that a label started (`platforms`, §5.6) runs every check, with a note: the
/// label is a deliberate request, and the rule for changes that touch no code does not apply.
#[must_use]
pub fn for_run(root: &Path, ci: Option<&CiEvent>) -> Scope {
    match ci {
        Some(CiEvent::Push(_)) => {
            every_because("this is a push, which has no pull request to narrow by")
        }
        Some(CiEvent::Other) => every_because(
            "this event is not a pull request (a manual or scheduled run, for example)",
        ),
        Some(CiEvent::PullRequest(pr)) if pr.action == "labeled" => every_because(LABEL_NOTE),
        Some(CiEvent::PullRequest(pr)) => with_lists(root, |git, paths| {
            decide(changes::of_pull_request_against_main(git, pr), paths)
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
    fn a_docs_only_scope_runs_only_commit_msg() {
        assert!(runs(&Scope::DocsOnly, Check::CommitMsg));
        assert!(!runs(&Scope::DocsOnly, Check::Clippy));
        assert!(!runs(&Scope::DocsOnly, Check::Ui));
        assert!(runs(&every(), Check::Clippy));
        assert!(runs(&every_because("x"), Check::Ui));
    }

    // AC: CE-08C.2
    #[test]
    fn docs_only_changes_on_a_docs_branch_need_only_commit_msg() {
        let scope = decide(Ok(changes("docs/x", &["docs/a.md", "README.md"])), &lists());
        assert_eq!(scope, Scope::DocsOnly);
    }

    /// The note of a scope that runs every check; fails the test for any other scope or no note.
    fn note_of(scope: &Scope) -> &str {
        match scope {
            Scope::Every { note: Some(note) } => note,
            other => panic!("expected Every with a note, got {other:?}"),
        }
    }

    // AC: CE-08C.2
    #[test]
    fn a_phase_branch_runs_every_check_with_a_note() {
        let scope = decide(Ok(changes("phase/P50-leaner-ci", &["docs/x.md"])), &lists());
        assert!(note_of(&scope).contains("phase branch"), "{scope:?}");
    }

    // AC: CE-08C.2
    #[test]
    fn a_malformed_phase_branch_runs_every_check_with_a_note() {
        let scope = decide(Ok(changes("phase/foo", &["docs/x.md"])), &lists());
        assert!(note_of(&scope).contains("phase branch"), "{scope:?}");
    }

    // AC: CE-08C.2
    #[test]
    fn the_design_system_the_ui_imports_is_code_but_other_design_files_are_not() {
        let scope = decide(
            Ok(changes("docs/x", &["design/theos-ui/tokens.css"])),
            &lists(),
        );
        assert_eq!(scope, every());
        let scope = decide(
            Ok(changes("docs/x", &["design/theos-ui/fonts/a.woff2"])),
            &lists(),
        );
        assert_eq!(scope, every());
        let scope = decide(Ok(changes("docs/x", &["design/notes/ideas.md"])), &lists());
        assert_eq!(scope, Scope::DocsOnly);
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
        let scope = decide(Err(error), &lists());
        assert!(note_of(&scope).contains("boom-text"), "{scope:?}");
    }

    // AC: CE-08C.2
    #[test]
    fn no_change_runs_every_check_with_a_note() {
        let scope = decide(Ok(changes("docs/x", &[])), &lists());
        assert!(note_of(&scope).contains("no changed path"), "{scope:?}");
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
    fn a_push_runs_every_check_with_a_note() -> TestResult {
        let dir = docs_repo()?;
        let event = CiEvent::Push(PushEvent {
            before: "a".to_owned(),
            after: "b".to_owned(),
        });
        let scope = for_run(dir.path(), Some(&event));
        assert!(note_of(&scope).contains("push"), "{scope:?}");
        Ok(())
    }

    // AC: CE-08C.2
    #[test]
    fn a_manual_or_scheduled_run_runs_every_check_with_a_note() -> TestResult {
        let dir = docs_repo()?;
        let scope = for_run(dir.path(), Some(&CiEvent::Other));
        assert!(note_of(&scope).contains("not a pull request"), "{scope:?}");
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
        let scope = for_run(dir.path(), None);
        assert!(note_of(&scope).contains("detached"), "{scope:?}");
        Ok(())
    }

    #[test]
    fn missing_lists_run_every_check_with_a_note() -> TestResult {
        let dir = init_repo()?;
        commit(dir.path(), "chore: initial")?;
        let scope = for_run(dir.path(), None);
        assert!(note_of(&scope).contains("non-code list"), "{scope:?}");
        Ok(())
    }

    /// A pull request event with `action`, from `base` to `head` on the branch `head_ref`.
    fn pull_request(action: &str, base: &str, head: &str, head_ref: &str) -> CiEvent {
        CiEvent::PullRequest(PullRequestEvent {
            action: action.to_owned(),
            number: 1,
            title: "docs: a".to_owned(),
            body: None,
            base_sha: base.to_owned(),
            head_sha: head.to_owned(),
            head_ref: head_ref.to_owned(),
            label: (action == "labeled").then(|| "windows".to_owned()),
        })
    }

    /// `docs_repo` with a branch `docs/x` that adds `docs/a.md`: its base and head commits.
    fn docs_branch(path: &Path) -> TestResult<(String, String)> {
        let base = git(path, &["rev-parse", "HEAD"])?;
        git(path, &["switch", "-q", "-c", "docs/x"])?;
        write(path, "docs/a.md", "a\n")?;
        let head = commit(path, "docs: a")?;
        Ok((base, head))
    }

    // AC: CE-08C.2
    #[test]
    fn a_pull_request_event_decides_from_its_commits() -> TestResult {
        let dir = docs_repo()?;
        let path = dir.path();
        let (base, head) = docs_branch(path)?;
        let event = |head_ref: &str| pull_request("opened", &base, &head, head_ref);
        assert_eq!(for_run(path, Some(&event("docs/x"))), Scope::DocsOnly);
        let phase = for_run(path, Some(&event("phase/P50-x")));
        assert!(note_of(&phase).contains("phase branch"), "{phase:?}");
        Ok(())
    }

    // AC: CE-08C.6
    #[test]
    fn a_label_run_runs_every_check_of_a_docs_only_pull_request() -> TestResult {
        let dir = docs_repo()?;
        let path = dir.path();
        let (base, head) = docs_branch(path)?;
        let labeled = pull_request("labeled", &base, &head, "docs/x");
        let scope = for_run(path, Some(&labeled));
        assert!(note_of(&scope).contains("label"), "{scope:?}");
        // The same pull request opened is docs-only: the label alone made the difference.
        let opened = pull_request("opened", &base, &head, "docs/x");
        assert_eq!(for_run(path, Some(&opened)), Scope::DocsOnly);
        Ok(())
    }

    // AC: CE-08C.2
    #[test]
    fn a_changed_tracked_code_file_on_a_docs_branch_runs_every_check() -> TestResult {
        let dir = docs_repo()?;
        let path = dir.path();
        write(path, "crates/a/src/lib.rs", "// a\n")?;
        let main = commit(path, "feat: a")?;
        git(path, &["update-ref", "refs/remotes/origin/main", &main])?;
        docs_branch(path)?;
        assert_eq!(for_run(path, None), Scope::DocsOnly);
        write(path, "crates/a/src/lib.rs", "// changed\n")?;
        assert_eq!(for_run(path, None), every(), "unstaged");
        git(path, &["add", "crates/a/src/lib.rs"])?;
        assert_eq!(for_run(path, None), every(), "staged");
        Ok(())
    }

    // AC: CE-08C.2
    #[test]
    fn a_retargeted_pull_request_is_judged_against_main() -> TestResult {
        let dir = docs_repo()?;
        let path = dir.path();
        git(path, &["switch", "-q", "-c", "feature/b"])?;
        write(path, "crates/a/src/lib.rs", "// a\n")?;
        let b_tip = commit(path, "feat: a")?;
        git(path, &["switch", "-q", "-c", "docs/x"])?;
        write(path, "docs/x.md", "x\n")?;
        let head = commit(path, "docs: x")?;
        // The event's base is B's tip: the paths since it are docs only, since main they are not.
        let event = pull_request("opened", &b_tip, &head, "docs/x");
        assert_eq!(for_run(path, Some(&event)), every());
        // The same head with main's commit as base gives the same verdict: a base change cannot
        // alter it.
        let main = git(path, &["rev-parse", "origin/main"])?;
        let event = pull_request("opened", &main, &head, "docs/x");
        assert_eq!(for_run(path, Some(&event)), every());
        Ok(())
    }

    // AC: CE-08C.2
    #[test]
    fn a_pull_request_without_origin_main_runs_every_check_with_a_note() -> TestResult {
        let dir = docs_repo()?;
        let path = dir.path();
        let (base, head) = docs_branch(path)?;
        git(path, &["update-ref", "-d", "refs/remotes/origin/main"])?;
        let event = pull_request("opened", &base, &head, "docs/x");
        let scope = for_run(path, Some(&event));
        assert!(note_of(&scope).contains("could not be read"), "{scope:?}");
        Ok(())
    }

    // AC: CE-08C.2
    #[test]
    fn a_pull_request_whose_head_is_missing_runs_every_check_with_a_note() -> TestResult {
        let dir = docs_repo()?;
        let path = dir.path();
        let (base, _) = docs_branch(path)?;
        let missing = "0123456789abcdef0123456789abcdef01234567";
        let event = pull_request("opened", &base, missing, "docs/x");
        assert!(
            matches!(for_run(path, Some(&event)), Scope::Every { note: Some(_) }),
            "a head that is not in the repository"
        );
        Ok(())
    }
}
