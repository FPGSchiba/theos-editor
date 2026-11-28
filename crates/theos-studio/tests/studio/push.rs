//! Push: the target in git's own order, git's message whole, and no repo lock.

use std::path::{Path, PathBuf};

use tempfile::TempDir;
use theos_studio::Workspace;
use theos_studio::git_write::{PushFailed, push};
use theos_vcs::Cancel;
use theos_vcs::push::{PushError, Pushed, Rejection};

use crate::common::{Error, TestResult, put, spawn_helper};
use crate::helper::HelperTask;
use crate::repo::Repo;

const T900: &str = "lore/truths/T-900.md";

struct Remotes {
    repo: Repo,
    _dir: TempDir,
    dir: PathBuf,
}

impl Remotes {
    fn bare(&self, name: &str) -> PathBuf {
        self.dir.join(format!("{name}.git"))
    }

    /// The commit `branch` names in the bare remote `name`, if it exists.
    fn tip(&self, name: &str, branch: &str) -> Option<String> {
        let output = std::process::Command::new("git")
            .args([
                "rev-parse",
                "-q",
                "--verify",
                &format!("refs/heads/{branch}"),
            ])
            .current_dir(self.bare(name))
            .output()
            .ok()?;
        output
            .status
            .success()
            .then(|| String::from_utf8_lossy(&output.stdout).trim().to_owned())
    }

    fn head(&self) -> Result<String, Error> {
        Ok(self.repo.run(&["rev-parse", "HEAD"])?.trim().to_owned())
    }
}

/// A repository with a bare remote for each of `names`, each holding `main` at the first commit,
/// none of them configured as an upstream. A new local commit follows.
fn with_remotes(names: &[&str]) -> Result<Remotes, Error> {
    let repo = Repo::committed("")?;
    let dir = tempfile::tempdir()?;
    let path = dir.path().to_path_buf();
    for name in names {
        let bare = path.join(format!("{name}.git"));
        let bare = bare.to_str().ok_or("path")?;
        repo.run(&["clone", "-q", "--bare", ".", bare])?;
        repo.run(&["remote", "add", name, bare])?;
    }
    put(repo.top(), T900, b"changed\n")?;
    repo.commit("local change")?;
    Ok(Remotes {
        repo,
        _dir: dir,
        dir: path,
    })
}

fn run_push(remotes: &Remotes) -> Result<Result<Pushed, PushFailed>, Error> {
    let mut workspace = Workspace::open(remotes.repo.top())?;
    Ok(push(&mut workspace, &remotes.repo.git()?, &Cancel::new()))
}

fn script_hook(bare: &Path, text: &str) -> TestResult {
    let hook = bare.join("hooks").join("pre-receive");
    std::fs::write(&hook, text)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&hook, std::fs::Permissions::from_mode(0o755))?;
    }
    Ok(())
}

// AC: CE-22.3
#[test]
fn a_refusing_remote_shows_the_hooks_line_in_the_error() -> TestResult {
    let remotes = with_remotes(&["origin"])?;
    script_hook(
        &remotes.bare("origin"),
        "#!/bin/sh\necho 'theos: the clue C-901 confirms a never truth' >&2\nexit 1\n",
    )?;
    let result = run_push(&remotes)?;
    let Err(error) = result else {
        return Err(format!("expected a rejection, got {result:?}").into());
    };
    let PushFailed::Push(PushError::Rejected { reason, message }) = &error else {
        return Err(format!("expected a rejection, got {error:?}").into());
    };
    assert_eq!(*reason, Rejection::RemoteRefused);
    let line = "theos: the clue C-901 confirms a never truth";
    assert!(message.contains(line), "{message}");
    let shown = error.to_string();
    assert!(shown.contains(line), "{shown}");
    assert!(shown.ends_with(message.as_str()), "{shown}");
    Ok(())
}

// AC: CE-22.3
#[test]
fn the_display_of_a_push_error_is_the_push_errors_own() {
    let inner = PushError::Failed {
        message: "fatal: no such remote".to_owned(),
    };
    let expected = inner.to_string();
    assert_eq!(PushFailed::Push(inner).to_string(), expected);
}

#[test]
fn a_push_remote_of_the_branch_wins_and_origin_is_untouched() -> TestResult {
    let remotes = with_remotes(&["origin", "fork"])?;
    remotes
        .repo
        .run(&["config", "branch.main.remote", "origin"])?;
    remotes
        .repo
        .run(&["config", "branch.main.merge", "refs/heads/main"])?;
    remotes
        .repo
        .run(&["config", "branch.main.pushRemote", "fork"])?;
    let before = remotes.tip("origin", "main");
    let pushed = run_push(&remotes)??;
    assert!(matches!(pushed, Pushed::Updated { .. }), "{pushed:?}");
    assert_eq!(remotes.tip("fork", "main"), Some(remotes.head()?));
    assert_eq!(remotes.tip("origin", "main"), before);
    Ok(())
}

#[test]
fn remote_push_default_is_used_the_same() -> TestResult {
    let remotes = with_remotes(&["origin", "fork"])?;
    remotes
        .repo
        .run(&["config", "remote.pushDefault", "fork"])?;
    let before = remotes.tip("origin", "main");
    run_push(&remotes)??;
    assert_eq!(remotes.tip("fork", "main"), Some(remotes.head()?));
    assert_eq!(remotes.tip("origin", "main"), before);
    Ok(())
}

#[test]
fn an_upstream_with_another_name_is_pushed_to_that_name() -> TestResult {
    let remotes = with_remotes(&["origin"])?;
    remotes
        .repo
        .run(&["config", "branch.main.remote", "origin"])?;
    remotes
        .repo
        .run(&["config", "branch.main.merge", "refs/heads/trunk"])?;
    run_push(&remotes)??;
    assert_eq!(remotes.tip("origin", "trunk"), Some(remotes.head()?));
    Ok(())
}

#[test]
fn without_an_upstream_the_only_remote_is_used_and_becomes_the_upstream() -> TestResult {
    let remotes = with_remotes(&["origin"])?;
    run_push(&remotes)??;
    assert_eq!(remotes.tip("origin", "main"), Some(remotes.head()?));
    assert_eq!(
        remotes.repo.run(&["config", "branch.main.remote"])?.trim(),
        "origin"
    );
    Ok(())
}

#[test]
fn without_an_upstream_two_remotes_name_none() -> TestResult {
    let remotes = with_remotes(&["origin", "fork"])?;
    let result = run_push(&remotes)?;
    let Err(PushFailed::NoUpstream { remotes: names }) = result else {
        return Err(format!("expected NoUpstream, got {result:?}").into());
    };
    assert_eq!(names, ["fork", "origin"]);
    Ok(())
}

#[test]
fn a_detached_head_has_no_branch_to_push() -> TestResult {
    let remotes = with_remotes(&["origin"])?;
    remotes.repo.run(&["checkout", "-q", "--detach"])?;
    let result = run_push(&remotes)?;
    assert!(matches!(result, Err(PushFailed::Detached)), "{result:?}");
    Ok(())
}

#[test]
fn the_push_takes_no_repo_lock() -> TestResult {
    let remotes = with_remotes(&["origin"])?;
    let scratch = tempfile::tempdir()?;
    let task = HelperTask::Hold {
        root: remotes.repo.top().to_path_buf(),
        purpose: "synthetic hold".to_owned(),
        ready: scratch.path().join("ready"),
        release: scratch.path().join("release"),
    };
    let mut child = spawn_helper(&task)?;
    child.wait_for(&scratch.path().join("ready"))?;
    let result = run_push(&remotes);
    std::fs::write(scratch.path().join("release"), "")?;
    child.wait()?;
    result??;
    assert_eq!(remotes.tip("origin", "main"), Some(remotes.head()?));
    Ok(())
}
