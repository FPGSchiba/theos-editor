//! `fetch` and the remote environment.

use std::path::{Path, PathBuf};

use theos_vcs::{Cancel, GitError, remote, repo, status};

use crate::common::Sandbox;

type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;

/// A repository with one commit, a bare clone of it as `origin`, and a second commit pushed to
/// the bare repository from another clone. Returns the first repository and the second commit.
fn with_origin(sandbox: &Sandbox) -> Result<(PathBuf, String)> {
    let top = sandbox.init("repo")?;
    Sandbox::write(&top.join("a.md"), "one\n")?;
    sandbox.commit(&top, "first")?;
    let bare = sandbox.path().join("origin.git");
    let bare_text = bare.to_str().ok_or("path")?;
    sandbox.run(
        sandbox.path(),
        &["clone", "-q", "--bare", "repo", bare_text],
    )?;
    sandbox.run(&top, &["remote", "add", "origin", bare_text])?;
    let other = sandbox.path().join("other");
    let other_text = other.to_str().ok_or("path")?;
    sandbox.run(sandbox.path(), &["clone", "-q", bare_text, other_text])?;
    Sandbox::write(&other.join("b.md"), "two\n")?;
    sandbox.commit(&other, "second")?;
    sandbox.run(&other, &["push", "-q", "origin", "main"])?;
    let id = sandbox.run(&other, &["rev-parse", "HEAD"])?;
    Ok((top, id.trim().to_owned()))
}

fn state(sandbox: &Sandbox, top: &Path) -> Result<(Vec<status::Entry>, String)> {
    let entries = status::status(&sandbox.git()?, top, &Cancel::new())?.entries;
    let index = sandbox.run(top, &["ls-files", "--stage"])?;
    Ok((entries, index))
}

// AC: CE-22.3
#[test]
fn fetch_updates_the_remote_ref_and_leaves_the_tree_and_index() -> Result {
    let sandbox = Sandbox::new()?;
    let (top, second) = with_origin(&sandbox)?;
    let before = state(&sandbox, &top)?;

    remote::fetch(&sandbox.git()?, &top, "origin", &Cancel::new())?;

    let fetched = sandbox.run(&top, &["rev-parse", "refs/remotes/origin/main"])?;
    assert_eq!(fetched.trim(), second);
    assert_eq!(state(&sandbox, &top)?, before);
    assert!(!top.join("b.md").exists());
    Ok(())
}

// AC: CE-22.3
#[test]
fn an_askpass_in_the_environment_is_never_run() -> Result {
    let sandbox = Sandbox::new()?;
    let top = sandbox.init("repo")?;
    let script = sandbox.path().join("askpass.sh");
    Sandbox::write(
        &script,
        "#!/bin/sh\necho asked > askpass-marker\necho secret\n",
    )?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755))?;
    }
    let script_text = script.to_str().ok_or("path")?;
    let git = sandbox
        .git()?
        .with_env(&[("GIT_ASKPASS", script_text), ("SSH_ASKPASS", script_text)]);
    sandbox.run(&top, &["remote", "add", "dead", "http://127.0.0.1:9/x.git"])?;

    let result = remote::fetch(&git, &top, "dead", &Cancel::new());

    assert!(matches!(result, Err(GitError::Failed { .. })), "{result:?}");
    assert!(!top.join("askpass-marker").exists());
    assert!(!sandbox.path().join("askpass-marker").exists());
    Ok(())
}

#[test]
fn a_fetch_cancelled_before_the_start_is_cancelled() -> Result {
    let sandbox = Sandbox::new()?;
    let (top, _) = with_origin(&sandbox)?;
    let cancel = Cancel::new();
    cancel.cancel();

    let result = remote::fetch(&sandbox.git()?, &top, "origin", &cancel);

    assert!(
        matches!(result, Err(GitError::Cancelled { .. })),
        "{result:?}"
    );
    Ok(())
}

#[test]
fn value_reads_a_setting_and_none_for_an_unset_one() -> Result {
    let sandbox = Sandbox::new()?;
    let top = sandbox.init("repo")?;
    sandbox.run(&top, &["config", "core.sshCommand", "ssh -i key"])?;
    let git = sandbox.git()?;

    assert_eq!(
        repo::value(&git, &top, "core.sshCommand")?.as_deref(),
        Some("ssh -i key")
    );
    assert_eq!(repo::value(&git, &top, "core.nothing")?, None);
    Ok(())
}
