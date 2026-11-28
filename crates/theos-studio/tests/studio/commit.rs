//! The checked commit, in real repositories.

use std::path::Path;
use std::time::Duration;

use theos_studio::git_write::{CommitError, Committed, commit};
use theos_studio::lock::LockError;
use theos_studio::{StagedCheck, Workspace};
use theos_vcs::commit::Trailer;
use theos_vcs::{CommitId, Git, repo};

use crate::common::{Error, TestResult, put, spawn_helper};
use crate::helper::HelperTask;
use crate::repo::{Repo, variant};

const C900: &str = "lore/clues/C-900.md";
const C901: &str = "lore/clues/C-901.md";
const T900: &str = "lore/truths/T-900.md";

/// The runner of `repo` with an identity, so that git can commit.
fn git_of(repo: &Repo) -> Result<Git, Error> {
    Ok(repo.git()?.with_env(&[
        ("GIT_AUTHOR_NAME", "Synthetic"),
        ("GIT_AUTHOR_EMAIL", "synthetic@example.invalid"),
        ("GIT_COMMITTER_NAME", "Synthetic"),
        ("GIT_COMMITTER_EMAIL", "synthetic@example.invalid"),
    ]))
}

fn head(repo: &Repo) -> Result<Option<CommitId>, Error> {
    Ok(repo::head(&repo.git()?, repo.top())?)
}

fn trailer() -> Trailer {
    Trailer {
        key: "Refs".to_owned(),
        value: "CE-22.1".to_owned(),
    }
}

/// Commits with the message `msg` and the trailer.
fn try_commit(repo: &Repo, msg: &str) -> Result<Result<Committed, CommitError>, Error> {
    let mut workspace = Workspace::open(repo.top())?;
    Ok(commit(&mut workspace, &git_of(repo)?, msg, &[trailer()]))
}

fn stage(repo: &Repo, path: &str, text: &str) -> Result<(), Error> {
    put(repo.top(), path, text.as_bytes())?;
    repo.run(&["add", "--", path])?;
    Ok(())
}

fn staged_paths(repo: &Repo) -> Result<String, Error> {
    repo.run(&["diff", "--cached", "--name-only"])
}

fn on_rule(check: &StagedCheck, rule: &str, path: &str) -> bool {
    check
        .findings
        .iter()
        .any(|f| f.rule == rule && f.path == path)
}

fn clean_edit() -> Result<String, Error> {
    let text = std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/projects/staged")
            .join(T900),
    )?;
    Ok(format!("{text}\nEdited.\n"))
}

// AC: CE-22.1
#[test]
fn a_staged_error_blocks_the_commit() -> TestResult {
    let repo = Repo::committed("")?;
    let before = head(&repo)?;
    stage(&repo, C901, &variant("C-901.md")?)?;
    let result = try_commit(&repo, "add a clue")?;
    let Err(CommitError::Blocked(check)) = result else {
        return Err(format!("expected Blocked, got {result:?}").into());
    };
    assert!(on_rule(&check, "lore/never-confirm", C901), "{check:?}");
    assert_eq!(head(&repo)?, before);
    assert_eq!(staged_paths(&repo)?.trim(), C901);
    Ok(())
}

// AC: CE-22.1
#[test]
fn warnings_do_not_block_the_commit() -> TestResult {
    let repo = Repo::committed("")?;
    let before = head(&repo)?;
    stage(&repo, T900, &clean_edit()?)?;
    let Committed { id, check } = try_commit(&repo, "edit a truth")??;
    assert!(check.warnings() > 0, "{check:?}");
    assert_eq!(check.errors(), 0);
    assert_ne!(head(&repo)?, before);
    assert_eq!(head(&repo)?, Some(id));
    let body = repo.run(&["log", "-1", "--format=%B"])?;
    assert!(body.starts_with("edit a truth\n"), "{body}");
    assert!(body.contains("Refs: CE-22.1"), "{body}");
    assert_eq!(staged_paths(&repo)?, "");
    Ok(())
}

// AC: CE-22.1
#[test]
fn an_error_only_in_the_working_tree_is_allowed() -> TestResult {
    let repo = Repo::committed("")?;
    stage(&repo, T900, &clean_edit()?)?;
    put(repo.top(), C901, variant("C-901.md")?.as_bytes())?;
    let done = try_commit(&repo, "edit a truth")??;
    assert_eq!(done.check.errors(), 0);
    assert!(
        repo.run(&["status", "--short"])?
            .contains("?? lore/clues/C-901.md")
    );
    Ok(())
}

// AC: CE-22.1
#[test]
fn a_staged_error_fixed_only_on_disk_still_blocks() -> TestResult {
    let repo = Repo::committed("")?;
    let before = head(&repo)?;
    stage(&repo, C901, &variant("C-901.md")?)?;
    std::fs::remove_file(repo.top().join(C901))?;
    let result = try_commit(&repo, "add a clue")?;
    assert!(matches!(result, Err(CommitError::Blocked(_))), "{result:?}");
    assert_eq!(head(&repo)?, before);
    Ok(())
}

// AC: CE-22.1
#[test]
fn a_staged_deletion_that_breaks_a_link_blocks() -> TestResult {
    let repo = Repo::committed("")?;
    let before = head(&repo)?;
    repo.run(&["rm", "-q", "--", T900])?;
    let result = try_commit(&repo, "drop a truth")?;
    let Err(CommitError::Blocked(check)) = result else {
        return Err(format!("expected Blocked, got {result:?}").into());
    };
    assert!(check.blocks(), "{check:?}");
    assert!(check.findings.iter().any(|f| f.path == C900), "{check:?}");
    assert_eq!(head(&repo)?, before);
    Ok(())
}

// AC: CE-22.1
#[test]
fn nothing_staged_is_refused() -> TestResult {
    let repo = Repo::committed("")?;
    put(repo.top(), T900, clean_edit()?.as_bytes())?;
    let result = try_commit(&repo, "edit a truth")?;
    assert!(
        matches!(result, Err(CommitError::NothingStaged)),
        "{result:?}"
    );
    Ok(())
}

// AC: CE-22.1
#[test]
fn an_empty_message_is_refused() -> TestResult {
    let repo = Repo::committed("")?;
    let before = head(&repo)?;
    stage(&repo, T900, &clean_edit()?)?;
    let result = try_commit(&repo, " \n\t ")?;
    assert!(matches!(result, Err(CommitError::Message(_))), "{result:?}");
    let text = result.err().map(|error| error.to_string());
    assert_eq!(text.as_deref(), Some("invalid commit message: it is empty"));
    assert_eq!(head(&repo)?, before);
    Ok(())
}

// AC: CE-22.1
#[test]
fn a_bad_trailer_is_a_message_error() -> TestResult {
    let repo = Repo::committed("")?;
    stage(&repo, T900, &clean_edit()?)?;
    let mut workspace = Workspace::open(repo.top())?;
    let bad = Trailer {
        key: "Not Valid".to_owned(),
        value: "x".to_owned(),
    };
    let result = commit(&mut workspace, &git_of(&repo)?, "edit", &[bad]);
    assert!(matches!(result, Err(CommitError::Message(_))), "{result:?}");
    Ok(())
}

// AC: CE-22.1
#[test]
fn a_conflict_is_refused_with_its_path() -> TestResult {
    let repo = Repo::committed("")?;
    let text = std::fs::read_to_string(repo.top().join(T900))?;
    repo.run(&["switch", "-q", "-c", "other"])?;
    put(repo.top(), T900, text.replace("A.", "B.").as_bytes())?;
    repo.commit("other")?;
    repo.run(&["switch", "-q", "main"])?;
    put(repo.top(), T900, text.replace("A.", "C.").as_bytes())?;
    repo.commit("main")?;
    // The merge stops in a conflict, so the command fails.
    let merged = repo.run(&[
        "-c",
        "user.name=Synthetic",
        "-c",
        "user.email=synthetic@example.invalid",
        "merge",
        "-q",
        "other",
    ]);
    assert!(merged.is_err(), "the merge did not conflict: {merged:?}");
    let result = try_commit(&repo, "conclude")?;
    let Err(CommitError::Conflicted(paths)) = result else {
        return Err(format!("expected Conflicted, got {result:?}").into());
    };
    assert_eq!(paths, [T900]);
    Ok(())
}

// AC: CE-22.1
#[test]
fn a_held_lock_is_busy_and_nothing_is_committed() -> TestResult {
    let repo = Repo::committed("")?;
    let before = head(&repo)?;
    stage(&repo, T900, &clean_edit()?)?;
    let mut workspace = Workspace::open(repo.top())?;
    workspace.set_lock_wait(Duration::from_millis(200));
    let scratch = tempfile::tempdir()?;
    let task = HelperTask::Hold {
        root: repo.top().to_path_buf(),
        purpose: "synthetic hold".to_owned(),
        ready: scratch.path().join("ready"),
        release: scratch.path().join("release"),
    };
    let mut child = spawn_helper(&task)?;
    child.wait_for(&scratch.path().join("ready"))?;
    let result = commit(&mut workspace, &git_of(&repo)?, "edit", &[]);
    assert!(
        matches!(&result, Err(CommitError::Lock(LockError::Busy { .. }))),
        "{result:?}"
    );
    assert_eq!(head(&repo)?, before);
    std::fs::write(scratch.path().join("release"), "")?;
    child.wait()?;
    commit(&mut workspace, &git_of(&repo)?, "edit", &[])?;
    Ok(())
}

// AC: CE-22.1
#[test]
fn a_file_staged_outside_the_workspace_is_judged_without_a_sync() -> TestResult {
    let repo = Repo::committed("")?;
    let before = head(&repo)?;
    let mut workspace = Workspace::open(repo.top())?;
    // Written and staged behind the workspace's back; nothing syncs its index.
    stage(&repo, C901, &variant("C-901.md")?)?;
    let result = commit(&mut workspace, &git_of(&repo)?, "add a clue", &[]);
    let Err(CommitError::Blocked(check)) = result else {
        return Err(format!("expected Blocked, got {result:?}").into());
    };
    assert!(on_rule(&check, "lore/never-confirm", C901), "{check:?}");
    assert_eq!(head(&repo)?, before);
    Ok(())
}
