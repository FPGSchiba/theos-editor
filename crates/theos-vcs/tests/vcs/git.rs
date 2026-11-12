//! The runner: stdout, failures, stdin, large answers and added environment.

use std::ffi::OsStr;

use theos_vcs::GitError;

use crate::common::Sandbox;

fn args<'a>(list: &'a [&'a str]) -> Vec<&'a OsStr> {
    list.iter().map(OsStr::new).collect()
}

#[test]
fn run_returns_stdout() -> Result<(), Box<dyn std::error::Error>> {
    let sandbox = Sandbox::new()?;
    let out = sandbox
        .git()?
        .run(&args(&["--version"]), sandbox.path(), None, &[])?;
    assert!(String::from_utf8(out)?.starts_with("git version"));
    Ok(())
}

#[test]
fn a_failing_command_is_failed_with_its_stderr() -> Result<(), Box<dyn std::error::Error>> {
    let sandbox = Sandbox::new()?;
    let err = sandbox
        .git()?
        .run(
            &args(&["rev-parse", "--show-toplevel"]),
            sandbox.path(),
            None,
            &[],
        )
        .err()
        .ok_or("expected a failure")?;
    match err {
        GitError::Failed { code, stderr, .. } => {
            assert_eq!(code, Some(128));
            assert!(stderr.contains("not a git repository"), "{stderr}");
        }
        other => return Err(format!("unexpected {other:?}").into()),
    }
    Ok(())
}

#[test]
fn stdin_reaches_git() -> Result<(), Box<dyn std::error::Error>> {
    let sandbox = Sandbox::new()?;
    let out = sandbox.git()?.run(
        &args(&["hash-object", "--stdin"]),
        sandbox.path(),
        Some(b"abc"),
        &[],
    )?;
    assert_eq!(
        String::from_utf8(out)?,
        "f2ba8f84ab5c1bce84a7b441cb1959cfc7093b7f\n"
    );
    Ok(())
}

#[test]
fn a_large_answer_does_not_block() -> Result<(), Box<dyn std::error::Error>> {
    let sandbox = Sandbox::new()?;
    let repo = sandbox.init("repo")?;
    let git = sandbox.git()?;
    let blob = vec![b'x'; 3 * 1024 * 1024];
    let id = git.run(
        &args(&["hash-object", "-w", "--stdin"]),
        &repo,
        Some(&blob),
        &[],
    )?;
    let id = String::from_utf8(id)?;
    // The batch input is written while the answer is read.
    let out = git.run(
        &args(&["cat-file", "--batch"]),
        &repo,
        Some(id.as_bytes()),
        &[],
    )?;
    assert!(out.len() > 3 * 1024 * 1024);
    Ok(())
}

#[test]
fn with_env_reaches_every_command() -> Result<(), Box<dyn std::error::Error>> {
    let sandbox = Sandbox::new()?;
    let git = sandbox.git()?.with_env(&[
        ("GIT_AUTHOR_NAME", "Synthetic Author"),
        ("GIT_AUTHOR_EMAIL", "a@example.invalid"),
    ]);
    let out = git.run(
        &args(&["var", "GIT_AUTHOR_IDENT"]),
        sandbox.path(),
        None,
        &[],
    )?;
    assert!(String::from_utf8(out)?.contains("Synthetic Author <a@example.invalid>"));
    Ok(())
}
