//! `commit` with trailers, hooks and the user's own settings.

use std::path::{Path, PathBuf};

use theos_vcs::commit::{CommitRefused, Trailer, commit};
use theos_vcs::{GitError, repo};

use crate::common::Sandbox;

type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;

fn staged_repo(sandbox: &Sandbox) -> Result<PathBuf> {
    let top = sandbox.init("repo")?;
    sandbox.run(&top, &["config", "user.name", "Synthetic"])?;
    sandbox.run(&top, &["config", "user.email", "synthetic@example.invalid"])?;
    Sandbox::write(&top.join("f.md"), "one\n")?;
    sandbox.run(&top, &["add", "-A"])?;
    Ok(top)
}

fn trailer(key: &str, value: &str) -> Trailer {
    Trailer {
        key: key.to_owned(),
        value: value.to_owned(),
    }
}

/// The stored message of HEAD, exactly (`%B` plus the newline `log` adds is cut off).
fn body(sandbox: &Sandbox, top: &Path) -> Result<String> {
    let text = sandbox.run(top, &["log", "-1", "--format=%B"])?;
    Ok(text.strip_suffix('\n').unwrap_or(&text).to_owned())
}

fn hook(top: &Path, script: &str) -> Result {
    let path = top.join(".git").join("hooks").join("pre-commit");
    Sandbox::write(&path, script)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755))?;
    }
    Ok(())
}

// AC: CE-22.1
#[test]
fn a_hash_line_stays_and_the_trailer_goes_last() -> Result {
    let sandbox = Sandbox::new()?;
    let top = staged_repo(&sandbox)?;
    let id = commit(
        &sandbox.git()?,
        &top,
        "F04: open the sky\n\n# hash line kept\n",
        &[trailer("Approved-by", "Jann")],
    )?;
    assert_eq!(
        body(&sandbox, &top)?,
        "F04: open the sky\n\n# hash line kept\n\nApproved-by: Jann\n"
    );
    assert_eq!(repo::head(&sandbox.git()?, &top)?, Some(id));
    Ok(())
}

// AC: CE-22.1
#[test]
fn commit_message_with_crlf_is_stored_with_lf() -> Result {
    let sandbox = Sandbox::new()?;
    let top = staged_repo(&sandbox)?;
    commit(
        &sandbox.git()?,
        &top,
        "a\r\n\r\nb\r\n",
        &[trailer("Approved-by", "Jann")],
    )?;
    assert_eq!(body(&sandbox, &top)?, "a\n\nb\n\nApproved-by: Jann\n");
    Ok(())
}

// AC: CE-22.1
#[test]
fn a_bad_message_or_trailer_commits_nothing() -> Result {
    let sandbox = Sandbox::new()?;
    let top = staged_repo(&sandbox)?;
    let git = sandbox.git()?;
    assert!(matches!(
        commit(&git, &top, " \r\n\t", &[]),
        Err(CommitRefused::Message(_))
    ));
    assert!(matches!(
        commit(&git, &top, "m", &[trailer("Approved by", "x")]),
        Err(CommitRefused::Trailer(_))
    ));
    assert!(matches!(
        commit(&git, &top, "m", &[trailer("", "x")]),
        Err(CommitRefused::Trailer(_))
    ));
    assert!(matches!(
        commit(&git, &top, "m", &[trailer("Key", "a\nb")]),
        Err(CommitRefused::Trailer(_))
    ));
    assert_eq!(repo::head(&git, &top)?, None);
    Ok(())
}

// AC: CE-22.1
#[test]
fn a_refusing_hook_fails_the_commit_with_its_text() -> Result {
    let sandbox = Sandbox::new()?;
    let top = staged_repo(&sandbox)?;
    hook(&top, "#!/bin/sh\necho 'theos: 1 error' >&2\nexit 1\n")?;
    let result = commit(&sandbox.git()?, &top, "m", &[]);
    match result {
        Err(CommitRefused::Git(GitError::Failed { stderr, .. })) => {
            assert!(stderr.contains("theos: 1 error"), "{stderr}");
        }
        other => panic!("expected Git(Failed), got {other:?}"),
    }
    assert_eq!(repo::head(&sandbox.git()?, &top)?, None);
    Ok(())
}

// AC: CE-22.1
#[test]
fn a_loud_refusing_hook_is_reported_whole() -> Result {
    let sandbox = Sandbox::new()?;
    let top = staged_repo(&sandbox)?;
    hook(
        &top,
        "#!/bin/sh\nhead -c 100000 /dev/zero | tr '\\0' 'x' >&2\nexit 1\n",
    )?;
    let result = commit(&sandbox.git()?, &top, "m", &[]);
    match result {
        Err(CommitRefused::Git(GitError::Failed { stderr, .. })) => {
            assert!(stderr.trim().len() >= 100_000, "{}", stderr.len());
        }
        other => panic!("expected Git(Failed), got {other:?}"),
    }
    assert_eq!(repo::head(&sandbox.git()?, &top)?, None);
    Ok(())
}

// AC: CE-22.1
#[test]
fn the_users_comment_char_and_verbose_change_nothing() -> Result {
    let sandbox = Sandbox::new()?;
    let top = staged_repo(&sandbox)?;
    sandbox.run(&top, &["config", "core.commentChar", ";"])?;
    sandbox.run(&top, &["config", "commit.verbose", "true"])?;
    commit(
        &sandbox.git()?,
        &top,
        "subject\n\n; semi line\n# hash line\n",
        &[trailer("Approved-by", "Jann")],
    )?;
    assert_eq!(
        body(&sandbox, &top)?,
        "subject\n\n; semi line\n# hash line\n\nApproved-by: Jann\n"
    );
    Ok(())
}
