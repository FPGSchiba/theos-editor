//! The studio's checked commit with P05's real `pre-commit` hook installed: the hook runs
//! `theos check --staged` in its own process while the studio holds the repo lock.

use std::path::Path;

use theos_studio::Workspace;
use theos_studio::git_write::{CommitError, commit};
use theos_vcs::{Git, repo};

use crate::common::{Error, Sandbox, TestResult, staged_project, variant};

const C901: &str = "lore/clues/C-901.md";
const T900: &str = "lore/truths/T-900.md";

/// The staged fixture project as a committed repository with the hook installed, and the runner
/// the studio commits with (isolated configuration and an identity).
fn hooked(sandbox: &Sandbox) -> Result<(std::path::PathBuf, Git), Error> {
    let top = sandbox.copy_project(&staged_project(), "repo")?;
    sandbox.commit_all(&top)?;
    let installed = sandbox.run(&["hook", "install"], &top)?;
    assert!(installed.status.success(), "{installed:?}");
    let config = sandbox.gitconfig();
    let config_dir = sandbox.config();
    let git = Git::find()?.with_env(&[
        (
            "GIT_CONFIG_GLOBAL",
            config.to_str().ok_or("config path is not UTF-8")?,
        ),
        ("GIT_CONFIG_NOSYSTEM", "1"),
        (
            "THEOS_CONFIG_DIR",
            config_dir.to_str().ok_or("config path is not UTF-8")?,
        ),
        ("GIT_AUTHOR_NAME", "Synthetic"),
        ("GIT_AUTHOR_EMAIL", "synthetic@example.invalid"),
        ("GIT_COMMITTER_NAME", "Synthetic"),
        ("GIT_COMMITTER_EMAIL", "synthetic@example.invalid"),
    ]);
    Ok((top, git))
}

fn stage(sandbox: &Sandbox, top: &Path, path: &str, text: &str) -> TestResult {
    let file = top.join(path);
    if let Some(folder) = file.parent() {
        std::fs::create_dir_all(folder)?;
    }
    std::fs::write(file, text)?;
    sandbox.git(&["add", "--", path], top)?;
    Ok(())
}

// This test is a missing-coverage test: it passes against the correct code as written.
// AC: CE-22.1
#[test]
fn a_warning_only_commit_passes_the_real_hook_while_the_studio_holds_the_lock() -> TestResult {
    let sandbox = Sandbox::new()?;
    let (top, git) = hooked(&sandbox)?;
    let before = repo::head(&git, &top)?;
    let text = std::fs::read_to_string(top.join(T900))?;
    stage(&sandbox, &top, T900, &format!("{text}\nEdited.\n"))?;
    let mut workspace = Workspace::open(&top)?;

    let done = commit(&mut workspace, &git, "edit a truth", &[])?;

    assert!(done.check.warnings() > 0, "{:?}", done.check);
    assert_eq!(done.check.errors(), 0);
    assert_ne!(repo::head(&git, &top)?, before);
    assert_eq!(repo::head(&git, &top)?, Some(done.id));
    Ok(())
}

// AC: CE-22.1
#[test]
fn a_staged_error_is_blocked_by_the_studio_before_git_runs() -> TestResult {
    let sandbox = Sandbox::new()?;
    let (top, git) = hooked(&sandbox)?;
    let before = repo::head(&git, &top)?;
    stage(&sandbox, &top, C901, &variant("C-901.md")?)?;
    let mut workspace = Workspace::open(&top)?;

    let result = commit(&mut workspace, &git, "add a clue", &[]);

    assert!(matches!(result, Err(CommitError::Blocked(_))), "{result:?}");
    assert_eq!(repo::head(&git, &top)?, before);
    Ok(())
}
