//! `push` against local bare repositories.

use std::path::{Path, PathBuf};

use theos_vcs::push::{PushError, PushTarget, Pushed, Rejection, push};
use theos_vcs::{Cancel, repo};

use crate::common::Sandbox;

type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;

fn target(remote: &str) -> PushTarget {
    PushTarget {
        remote: remote.to_owned(),
        branch: "main".to_owned(),
        set_upstream: false,
    }
}

/// A repository with one commit and an empty bare repository as `origin`.
fn with_bare(sandbox: &Sandbox) -> Result<(PathBuf, PathBuf)> {
    let top = sandbox.init("repo")?;
    Sandbox::write(&top.join("a.md"), "one\n")?;
    sandbox.commit(&top, "first")?;
    let bare = sandbox.path().join("origin.git");
    let text = bare.to_str().ok_or("path")?;
    sandbox.run(
        sandbox.path(),
        &["init", "-q", "--bare", "-b", "main", text],
    )?;
    sandbox.run(&top, &["remote", "add", "origin", text])?;
    Ok((top, bare))
}

/// The stderr of the same push run by the test itself: a failed push changes nothing, so git
/// answers the same way.
fn independent_stderr(sandbox: &Sandbox, top: &Path, remote: &str) -> Result<String> {
    sandbox.failing_stderr(
        top,
        &["push", "--porcelain", remote, "HEAD:refs/heads/main"],
        &[],
    )
}

/// A failure before any ref was judged, whose message is git's stderr, whole and last in the
/// error's text.
pub(crate) fn assert_failed_with(
    result: std::result::Result<Pushed, PushError>,
    independent: &str,
) -> Result {
    let error = result.err().ok_or("the push succeeded")?;
    let text = error.to_string();
    match error {
        PushError::Failed { message } => {
            assert!(text.ends_with(&message), "{text}");
            assert_eq!(message, independent);
            Ok(())
        }
        other => Err(format!("{other:?}").into()),
    }
}

/// A refusal of `reason` whose message is git's stderr, whole and last in the error's text.
fn assert_git_message(
    result: std::result::Result<Pushed, PushError>,
    reason: Rejection,
    independent: &str,
) -> Result<String> {
    let error = result.err().ok_or("the push succeeded")?;
    let text = error.to_string();
    match error {
        PushError::Rejected {
            reason: got,
            message,
        } => {
            assert_eq!(got, reason);
            assert!(text.ends_with(&message), "{text}");
            assert_eq!(message, independent);
            Ok(message)
        }
        other => Err(format!("{other:?}").into()),
    }
}

// AC: CE-22.3
#[test]
fn a_new_branch_then_an_update_then_up_to_date() -> Result {
    let sandbox = Sandbox::new()?;
    let (top, bare) = with_bare(&sandbox)?;
    let git = sandbox.git()?;
    let first = repo::head(&git, &top)?.ok_or("head")?;

    let pushed = push(&git, &top, &target("origin"), &Cancel::new())?;
    assert_eq!(
        pushed,
        Pushed::Updated {
            from: None,
            to: first.clone()
        }
    );
    assert_eq!(sandbox.run(&bare, &["rev-parse", "main"])?.trim(), first.0);

    Sandbox::write(&top.join("b.md"), "two\n")?;
    sandbox.commit(&top, "second")?;
    let second = repo::head(&git, &top)?.ok_or("head")?;
    let pushed = push(&git, &top, &target("origin"), &Cancel::new())?;
    assert_eq!(
        pushed,
        Pushed::Updated {
            from: Some(first),
            to: second
        }
    );

    let pushed = push(&git, &top, &target("origin"), &Cancel::new())?;
    assert_eq!(pushed, Pushed::UpToDate);
    Ok(())
}

// AC: CE-22.3
#[test]
fn a_hook_refusal_shows_the_hooks_line() -> Result {
    let sandbox = Sandbox::new()?;
    let (top, bare) = with_bare(&sandbox)?;
    Sandbox::write(
        &bare.join("hooks").join("pre-receive"),
        "#!/bin/sh\necho 'theos: the clue C-901 confirms a never truth' >&2\nexit 1\n",
    )?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(
            bare.join("hooks").join("pre-receive"),
            std::fs::Permissions::from_mode(0o755),
        )?;
    }

    let result = push(&sandbox.git()?, &top, &target("origin"), &Cancel::new());

    let independent = independent_stderr(&sandbox, &top, "origin")?;
    let message = assert_git_message(result, Rejection::RemoteRefused, &independent)?;
    assert!(
        message.contains("theos: the clue C-901 confirms a never truth"),
        "{message}"
    );
    Ok(())
}

// AC: CE-22.3
#[test]
fn a_remote_that_is_ahead_is_not_a_fast_forward_with_gits_stderr() -> Result {
    let sandbox = Sandbox::new()?;
    let (top, bare) = with_bare(&sandbox)?;
    let git = sandbox.git()?;
    push(&git, &top, &target("origin"), &Cancel::new())?;
    let bare_text = bare.to_str().ok_or("path")?;
    let other = sandbox.path().join("other");
    let other_text = other.to_str().ok_or("path")?;
    sandbox.run(sandbox.path(), &["clone", "-q", bare_text, other_text])?;
    Sandbox::write(&other.join("b.md"), "two\n")?;
    sandbox.commit(&other, "ahead")?;
    sandbox.run(&other, &["push", "-q", "origin", "main"])?;
    Sandbox::write(&top.join("c.md"), "three\n")?;
    sandbox.commit(&top, "mine")?;

    let result = push(&git, &top, &target("origin"), &Cancel::new());

    let independent = independent_stderr(&sandbox, &top, "origin")?;
    assert_git_message(result, Rejection::NotFastForward, &independent)?;
    Ok(())
}

// AC: CE-22.3
#[test]
fn a_missing_remote_fails_with_gits_text() -> Result {
    let sandbox = Sandbox::new()?;
    let (top, _) = with_bare(&sandbox)?;
    let missing = sandbox.path().join("nope.git");
    let missing_text = missing.to_str().ok_or("path")?;

    let result = push(&sandbox.git()?, &top, &target(missing_text), &Cancel::new());

    let independent = independent_stderr(&sandbox, &top, missing_text)?;
    assert_failed_with(result, &independent)
}

// AC: CE-22.3
#[test]
fn set_upstream_records_the_branch() -> Result {
    let sandbox = Sandbox::new()?;
    let (top, _) = with_bare(&sandbox)?;
    let mut wanted = target("origin");
    wanted.set_upstream = true;

    push(&sandbox.git()?, &top, &wanted, &Cancel::new())?;

    assert_eq!(
        sandbox.run(&top, &["config", "branch.main.remote"])?.trim(),
        "origin"
    );
    assert_eq!(
        sandbox.run(&top, &["config", "branch.main.merge"])?.trim(),
        "refs/heads/main"
    );
    Ok(())
}
