//! Pull and abort: two clones of the staged fixture project and a local bare remote.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use tempfile::TempDir;
use theos_studio::git_write::{PullError, Pulled, abort_merge, commit, pull};
use theos_studio::lock::LockError;
use theos_studio::{Changes, RecordChange, StudioError, Workspace};
use theos_vcs::merge::merge_state;
use theos_vcs::{Cancel, CommitId, GitError, repo};

use crate::common::{Error, TestResult, cancel_when_exists, put, sh_path, spawn_helper};
use crate::helper::HelperTask;
use crate::repo::{Repo, variant};

const C901: &str = "lore/clues/C-901.md";
const T900: &str = "lore/truths/T-900.md";
const T901: &str = "lore/truths/T-901.md";
const T902: &str = "lore/truths/T-902.md";

/// A bare `origin` and two clones of it: `a` pulls, `b` pushes what `a` pulls.
struct Remote {
    a: Repo,
    b: Repo,
    /// The bare repository.
    bare: PathBuf,
    seed: Repo,
    _bare: TempDir,
}

fn remote() -> Result<Remote, Error> {
    let seed = Repo::committed("")?;
    let bare_dir = tempfile::tempdir()?;
    let bare: PathBuf = bare_dir.path().join("origin.git");
    seed.run(&["clone", "-q", "--bare", ".", bare.to_str().ok_or("path")?])?;
    Ok(Remote {
        a: Repo::clone_of(&bare)?,
        b: Repo::clone_of(&bare)?,
        bare,
        seed,
        _bare: bare_dir,
    })
}

/// Writes `text` to `path` in `repo`, commits everything and pushes it.
fn push_change(repo: &Repo, path: &str, text: &str) -> TestResult {
    put(repo.top(), path, text.as_bytes())?;
    repo.commit("upstream change")?;
    repo.run(&["push", "-q", "origin", "main"])?;
    Ok(())
}

fn read(repo: &Repo, path: &str) -> Result<String, Error> {
    Ok(std::fs::read_to_string(repo.top().join(path))?)
}

/// The text of `path` in `repo` with one line added to its body.
fn edited(repo: &Repo, path: &str, line: &str) -> Result<String, Error> {
    Ok(format!("{}\n{line}\n", read(repo, path)?))
}

fn head(repo: &Repo) -> Result<Option<CommitId>, Error> {
    Ok(repo::head(&repo.git()?, repo.top())?)
}

fn parents(repo: &Repo) -> Result<usize, Error> {
    let line = repo.run(&["rev-list", "--parents", "-n", "1", "HEAD"])?;
    Ok(line.split_whitespace().count() - 1)
}

fn try_pull(repo: &Repo) -> Result<Result<Pulled, PullError>, Error> {
    let mut workspace = Workspace::open(repo.top())?;
    Ok(pull(&mut workspace, &repo.git()?, &Cancel::new()))
}

fn record<'a>(changes: &'a Changes, id: &str) -> Result<&'a RecordChange, Error> {
    changes
        .records
        .iter()
        .find(|record| record.id == id)
        .ok_or_else(|| format!("no record {id} in {changes:?}").into())
}

/// A new truth, made from T-901 with another id (synthetic, as the fixture).
fn new_truth(repo: &Repo) -> Result<String, Error> {
    Ok(read(repo, T901)?
        .replace("id: T-901", "id: T-902")
        .replace("Synthetic truth B", "Synthetic truth C"))
}

/// `a` has a commit of its own on T-901; `b` pushed `path` with `text`.
fn diverged(remote: &Remote, path: &str, text: &str) -> TestResult {
    push_change(&remote.b, path, text)?;
    put(
        remote.a.top(),
        T901,
        edited(&remote.a, T901, "Local.")?.as_bytes(),
    )?;
    remote.a.commit("local change")?;
    Ok(())
}

#[test]
fn behind_fast_forwards_and_the_index_sees_the_pulled_record() -> TestResult {
    let remote = remote()?;
    push_change(&remote.b, T902, &new_truth(&remote.b)?)?;
    let mut workspace = Workspace::open(remote.a.top())?;
    assert_eq!(workspace.index().record("T-902")?.len(), 0);
    let pulled = pull(&mut workspace, &remote.a.git()?, &Cancel::new())?;
    let Pulled::FastForward { to } = pulled else {
        return Err(format!("expected FastForward, got {pulled:?}").into());
    };
    assert_eq!(head(&remote.a)?, Some(to));
    assert_eq!(head(&remote.a)?, head(&remote.b)?);
    assert_eq!(workspace.index().record("T-902")?.len(), 1);
    Ok(())
}

#[test]
fn nothing_upstream_is_up_to_date() -> TestResult {
    let remote = remote()?;
    let pulled = try_pull(&remote.a)??;
    assert!(matches!(pulled, Pulled::UpToDate), "{pulled:?}");
    Ok(())
}

#[test]
fn a_clean_diverged_pull_is_merged_with_two_parents_and_the_warnings() -> TestResult {
    let remote = remote()?;
    diverged(&remote, T900, &edited(&remote.b, T900, "Remote.")?)?;
    let pulled = try_pull(&remote.a)??;
    let Pulled::Merged { id, check } = pulled else {
        return Err(format!("expected Merged, got {pulled:?}").into());
    };
    assert_eq!(check.errors(), 0);
    assert!(check.warnings() > 0, "{check:?}");
    assert_eq!(head(&remote.a)?, Some(id));
    assert_eq!(parents(&remote.a)?, 2);
    assert!(merge_state(&remote.a.git()?, remote.a.top())?.is_none());
    assert!(read(&remote.a, T900)?.contains("Remote."));
    Ok(())
}

#[test]
fn an_error_in_the_merged_result_is_refused_and_the_merge_waits() -> TestResult {
    let remote = remote()?;
    diverged(&remote, C901, &variant("C-901.md")?)?;
    let before = head(&remote.a)?;
    let mut workspace = Workspace::open(remote.a.top())?;
    let git = remote.a.git()?;
    let pulled = pull(&mut workspace, &git, &Cancel::new())?;
    let Pulled::Refused { check, changes } = pulled else {
        return Err(format!("expected Refused, got {pulled:?}").into());
    };
    assert!(
        check
            .findings
            .iter()
            .any(|f| f.rule == "lore/never-confirm" && f.path == C901),
        "{check:?}"
    );
    assert!(
        changes.records.iter().any(|r| r.id == "C-901"),
        "{changes:?}"
    );
    assert!(merge_state(&git, remote.a.top())?.is_some());
    assert_eq!(head(&remote.a)?, before);

    let fixed = variant("C-901.md")?.replace("reveals: T-901", "reveals: T-900");
    put(remote.a.top(), C901, fixed.as_bytes())?;
    remote.a.run(&["add", "--", C901])?;
    let done = commit(&mut workspace, &git, "conclude the pull", &[])?;
    assert_eq!(head(&remote.a)?, Some(done.id));
    assert_eq!(parents(&remote.a)?, 2);
    assert!(merge_state(&git, remote.a.top())?.is_none());
    Ok(())
}

#[test]
fn a_conflict_is_reported_by_record_and_abort_restores_head() -> TestResult {
    let remote = remote()?;
    let text = read(&remote.a, T900)?;
    push_change(
        &remote.b,
        T900,
        &text.replace("statement A.", "statement Remote."),
    )?;
    put(
        remote.a.top(),
        T900,
        text.replace("statement A.", "statement Local.").as_bytes(),
    )?;
    remote.a.commit("local change")?;
    let before = head(&remote.a)?;
    let mut workspace = Workspace::open(remote.a.top())?;
    let git = remote.a.git()?;
    let pulled = pull(&mut workspace, &git, &Cancel::new())?;
    let Pulled::Conflicts(changes) = pulled else {
        return Err(format!("expected Conflicts, got {pulled:?}").into());
    };
    assert!(record(&changes, "T-900")?.conflict.is_some(), "{changes:?}");
    assert!(merge_state(&git, remote.a.top())?.is_some());

    abort_merge(&mut workspace, &git)?;
    assert_eq!(head(&remote.a)?, before);
    assert!(merge_state(&git, remote.a.top())?.is_none());
    assert!(read(&remote.a, T900)?.contains("statement Local."));
    Ok(())
}

#[test]
fn abort_without_a_merge_is_refused() -> TestResult {
    let remote = remote()?;
    let mut workspace = Workspace::open(remote.a.top())?;
    let result = abort_merge(&mut workspace, &remote.a.git()?);
    assert!(matches!(result, Err(StudioError::Git(_))), "{result:?}");
    Ok(())
}

#[test]
fn an_unstaged_edit_of_a_record_the_upstream_changed_is_in_the_way() -> TestResult {
    let remote = remote()?;
    push_change(&remote.b, T900, &edited(&remote.b, T900, "Remote.")?)?;
    let mine = edited(&remote.a, T900, "Mine.")?;
    put(remote.a.top(), T900, mine.as_bytes())?;
    let before = head(&remote.a)?;
    let result = try_pull(&remote.a)?;
    let Err(PullError::LocalChanges { records, files }) = result else {
        return Err(format!("expected LocalChanges, got {result:?}").into());
    };
    let ids: Vec<&str> = records.iter().map(|r| r.id.as_str()).collect();
    assert_eq!(ids, ["T-900"]);
    assert!(files.is_empty(), "{files:?}");
    assert_eq!(head(&remote.a)?, before);
    assert_eq!(read(&remote.a, T900)?, mine);
    Ok(())
}

#[test]
fn an_untracked_file_the_upstream_adds_is_in_the_way() -> TestResult {
    let remote = remote()?;
    push_change(&remote.b, "notes.txt", "remote\n")?;
    put(remote.a.top(), "notes.txt", b"local\n")?;
    let result = try_pull(&remote.a)?;
    let Err(PullError::LocalChanges { records, files }) = result else {
        return Err(format!("expected LocalChanges, got {result:?}").into());
    };
    assert!(records.is_empty(), "{records:?}");
    assert_eq!(files.len(), 1, "{files:?}");
    Ok(())
}

#[test]
fn a_staged_file_blocks_a_merge_that_is_no_fast_forward() -> TestResult {
    let remote = remote()?;
    diverged(&remote, T900, &edited(&remote.b, T900, "Remote.")?)?;
    // C-900 is a file the upstream did not touch.
    let path = "lore/clues/C-900.md";
    put(
        remote.a.top(),
        path,
        edited(&remote.a, path, "Staged.")?.as_bytes(),
    )?;
    remote.a.run(&["add", "--", path])?;
    let before = head(&remote.a)?;
    let result = try_pull(&remote.a)?;
    let Err(PullError::LocalChanges { records, .. }) = result else {
        return Err(format!("expected LocalChanges, got {result:?}").into());
    };
    let ids: Vec<&str> = records.iter().map(|r| r.id.as_str()).collect();
    assert_eq!(ids, ["C-900"]);
    assert_eq!(head(&remote.a)?, before);
    Ok(())
}

#[test]
fn an_unrelated_unstaged_edit_survives_a_fast_forward() -> TestResult {
    let remote = remote()?;
    push_change(&remote.b, T900, &edited(&remote.b, T900, "Remote.")?)?;
    let mine = edited(&remote.a, T901, "Mine.")?;
    put(remote.a.top(), T901, mine.as_bytes())?;
    let pulled = try_pull(&remote.a)??;
    assert!(matches!(pulled, Pulled::FastForward { .. }), "{pulled:?}");
    assert_eq!(read(&remote.a, T901)?, mine);
    assert!(read(&remote.a, T900)?.contains("Remote."));
    Ok(())
}

#[test]
fn a_detached_head_is_refused() -> TestResult {
    let remote = remote()?;
    remote.a.run(&["switch", "-q", "--detach"])?;
    let result = try_pull(&remote.a)?;
    assert!(matches!(result, Err(PullError::Detached)), "{result:?}");
    Ok(())
}

#[test]
fn a_branch_without_an_upstream_is_refused() -> TestResult {
    let remote = remote()?;
    remote.a.run(&["branch", "--unset-upstream"])?;
    let result = try_pull(&remote.a)?;
    assert!(
        matches!(&result, Err(PullError::NoUpstream { branch }) if branch == "main"),
        "{result:?}"
    );
    Ok(())
}

#[test]
fn a_gone_upstream_is_no_upstream() -> TestResult {
    let remote = remote()?;
    // A bare remote refuses a push that deletes its current branch, so the branch is deleted
    // in the remote itself.
    let git_dir = format!("--git-dir={}", remote.bare.to_str().ok_or("path")?);
    remote
        .seed
        .run(&[&git_dir, "update-ref", "-d", "refs/heads/main"])?;
    remote.a.run(&["fetch", "-q", "--prune", "origin"])?;
    let result = try_pull(&remote.a)?;
    assert!(
        matches!(&result, Err(PullError::NoUpstream { branch }) if branch == "main"),
        "{result:?}"
    );
    Ok(())
}

#[test]
fn a_merge_in_progress_is_refused() -> TestResult {
    let remote = remote()?;
    let text = read(&remote.a, T900)?;
    push_change(
        &remote.b,
        T900,
        &text.replace("statement A.", "statement Remote."),
    )?;
    put(
        remote.a.top(),
        T900,
        text.replace("statement A.", "statement Local.").as_bytes(),
    )?;
    remote.a.commit("local change")?;
    remote.a.run(&["fetch", "-q", "origin"])?;
    let merged = remote.a.run(&["merge", "-q", "origin/main"]);
    assert!(merged.is_err(), "the merge did not conflict: {merged:?}");
    let result = try_pull(&remote.a)?;
    assert!(
        matches!(result, Err(PullError::MergeInProgress)),
        "{result:?}"
    );
    Ok(())
}

/// Starts a helper process that holds the repo lock of `root` until `release` exists.
fn hold(root: &Path, scratch: &Path) -> Result<crate::common::HelperProcess, Error> {
    let task = HelperTask::Hold {
        root: root.to_path_buf(),
        purpose: "synthetic hold".to_owned(),
        ready: scratch.join("ready"),
        release: scratch.join("release"),
    };
    let mut child = spawn_helper(&task)?;
    child.wait_for(&scratch.join("ready"))?;
    Ok(child)
}

#[test]
fn a_cancelled_fetch_takes_no_lock() -> TestResult {
    let remote = remote()?;
    push_change(&remote.b, T900, &edited(&remote.b, T900, "Remote.")?)?;
    let mut workspace = Workspace::open(remote.a.top())?;
    let wait = Duration::from_secs(10);
    workspace.set_lock_wait(wait);
    let scratch = tempfile::tempdir()?;
    let mut child = hold(remote.a.top(), scratch.path())?;
    let cancel = Cancel::new();
    cancel.cancel();
    let start = Instant::now();
    let result = pull(&mut workspace, &remote.a.git()?, &cancel);
    let took = start.elapsed();
    std::fs::write(scratch.path().join("release"), "")?;
    child.wait()?;
    assert!(
        matches!(&result, Err(PullError::Git(GitError::Cancelled { .. }))),
        "{result:?}"
    );
    assert!(took < wait / 2, "the pull waited {took:?}");
    assert!(!read(&remote.a, T900)?.contains("Remote."));
    Ok(())
}

#[test]
fn a_pull_waits_for_the_lock_then_fails_busy() -> TestResult {
    let remote = remote()?;
    push_change(&remote.b, T900, &edited(&remote.b, T900, "Remote.")?)?;
    let mut workspace = Workspace::open(remote.a.top())?;
    workspace.set_lock_wait(Duration::from_millis(200));
    let scratch = tempfile::tempdir()?;
    let mut child = hold(remote.a.top(), scratch.path())?;
    let result = pull(&mut workspace, &remote.a.git()?, &Cancel::new());
    std::fs::write(scratch.path().join("release"), "")?;
    child.wait()?;
    assert!(
        matches!(&result, Err(PullError::Lock(LockError::Busy { .. }))),
        "{result:?}"
    );
    assert!(!read(&remote.a, T900)?.contains("Remote."));
    Ok(())
}

#[test]
fn a_fetch_cancelled_while_it_runs_returns_cancelled_at_once() -> TestResult {
    let remote = remote()?;
    push_change(&remote.b, T900, &edited(&remote.b, T900, "Remote.")?)?;
    // The command that runs git-upload-pack on the remote: it signals and then hangs.
    let marker = remote.a.top().join(".git").join("fetch-started");
    let command = format!("touch '{}'; sleep 30 #", sh_path(&marker));
    remote
        .a
        .run(&["config", "remote.origin.uploadpack", &command])?;
    let mut workspace = Workspace::open(remote.a.top())?;
    let git = remote.a.git()?;
    let cancel = Cancel::new();
    let watcher = cancel_when_exists(marker, cancel.clone());
    let result = pull(&mut workspace, &git, &cancel);
    let returned = Instant::now();
    let cancelled_at = watcher
        .join()
        .map_err(|_| "watcher panicked")?
        .ok_or("the fetch never started")?;
    assert!(
        matches!(&result, Err(PullError::Git(GitError::Cancelled { .. }))),
        "{result:?}"
    );
    let took = returned.saturating_duration_since(cancelled_at);
    assert!(took < Duration::from_secs(2), "took {took:?}");
    Ok(())
}
