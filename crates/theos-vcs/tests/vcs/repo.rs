//! The repository queries.

use std::fs::canonicalize;

use theos_vcs::{Folding, GitError, repo};

use crate::common::Sandbox;

#[test]
fn top_of_a_subfolder_is_the_repository_folder() -> Result<(), Box<dyn std::error::Error>> {
    let sandbox = Sandbox::new()?;
    let top = sandbox.init("repo")?;
    let sub = top.join("a").join("b");
    std::fs::create_dir_all(&sub)?;
    let found = repo::top(&sandbox.git()?, &sub)?;
    assert_eq!(canonicalize(found)?, canonicalize(&top)?);
    Ok(())
}

#[test]
fn place_is_empty_at_the_top_and_a_slashed_path_below() -> Result<(), Box<dyn std::error::Error>> {
    let sandbox = Sandbox::new()?;
    let top = sandbox.init("repo")?;
    let sub = top.join("content").join("Inhalt ä b");
    std::fs::create_dir_all(&sub)?;
    assert_eq!(repo::place(&top, &top)?, "");
    assert_eq!(repo::place(&top, &sub)?, "content/Inhalt ä b");
    Ok(())
}

#[test]
fn place_outside_the_top_is_output() -> Result<(), Box<dyn std::error::Error>> {
    let sandbox = Sandbox::new()?;
    let top = sandbox.init("repo")?;
    let other = sandbox.path().join("other");
    std::fs::create_dir_all(&other)?;
    assert!(matches!(
        repo::place(&top, &other),
        Err(GitError::Output { .. })
    ));
    assert!(matches!(
        repo::place(&top, &top.join("missing")),
        Err(GitError::Io { .. })
    ));
    Ok(())
}

#[test]
fn top_outside_a_repository_fails() -> Result<(), Box<dyn std::error::Error>> {
    let sandbox = Sandbox::new()?;
    let result = repo::top(&sandbox.git()?, sandbox.path());
    assert!(matches!(result, Err(GitError::Failed { .. })));
    Ok(())
}

#[test]
fn a_flag_is_its_boolean_and_false_when_unset() -> Result<(), Box<dyn std::error::Error>> {
    let sandbox = Sandbox::new()?;
    let top = sandbox.init("repo")?;
    let git = sandbox.git()?;
    sandbox.run(&top, &["config", "core.ignorecase", "yes"])?;
    sandbox.run(&top, &["config", "core.precomposeunicode", "false"])?;
    assert!(repo::flag(&git, &top, "core.ignorecase")?);
    assert!(!repo::flag(&git, &top, "core.precomposeunicode")?);
    assert!(!repo::flag(&git, &top, "theos.unset")?);
    assert_eq!(
        Folding::read(&git, &top)?,
        Folding {
            ignore_case: true,
            precompose: false,
        }
    );
    sandbox.run(&top, &["config", "core.ignorecase", "maybe"])?;
    assert!(matches!(
        repo::flag(&git, &top, "core.ignorecase"),
        Err(GitError::Failed { .. })
    ));
    assert!(matches!(
        Folding::read(&git, &top),
        Err(GitError::Failed { .. })
    ));
    Ok(())
}

#[test]
fn hooks_dir_is_dot_git_hooks_by_default() -> Result<(), Box<dyn std::error::Error>> {
    let sandbox = Sandbox::new()?;
    let top = sandbox.init("repo")?;
    let hooks = repo::hooks_dir(&sandbox.git()?, &top)?;
    assert_eq!(
        canonicalize(hooks)?,
        canonicalize(top.join(".git").join("hooks"))?
    );
    Ok(())
}

#[test]
fn hooks_dir_honours_a_local_hooks_path() -> Result<(), Box<dyn std::error::Error>> {
    let sandbox = Sandbox::new()?;
    let top = sandbox.init("repo")?;
    sandbox.run(&top, &["config", "core.hooksPath", "githooks"])?;
    let hooks = repo::hooks_dir(&sandbox.git()?, &top)?;
    // The folder does not exist, so compare its parent and its name.
    let parent = hooks.parent().ok_or("no parent")?;
    assert_eq!(canonicalize(parent)?, canonicalize(&top)?);
    assert_eq!(hooks.file_name(), Some(std::ffi::OsStr::new("githooks")));
    Ok(())
}

#[test]
fn a_linked_worktree_shares_the_common_hooks() -> Result<(), Box<dyn std::error::Error>> {
    let sandbox = Sandbox::new()?;
    let top = sandbox.init("repo")?;
    Sandbox::write(&top.join("file.txt"), "synthetic\n")?;
    sandbox.commit(&top, "synthetic")?;
    let linked = sandbox.path().join("linked");
    sandbox.run(
        &top,
        &["worktree", "add", "-q", linked.to_str().ok_or("path")?],
    )?;
    let git = sandbox.git()?;
    let common = repo::common_dir(&git, &linked)?;
    let hooks = repo::hooks_dir(&git, &linked)?;
    assert_eq!(canonicalize(&common)?, canonicalize(top.join(".git"))?);
    assert_eq!(canonicalize(hooks)?, canonicalize(common.join("hooks"))?);
    Ok(())
}

#[test]
fn the_first_parent_of_a_root_commit_is_none() -> Result<(), Box<dyn std::error::Error>> {
    let sandbox = Sandbox::new()?;
    let top = sandbox.init("repo")?;
    Sandbox::write(&top.join("file.txt"), "synthetic\n")?;
    sandbox.commit(&top, "first")?;
    let first = sandbox.run(&top, &["rev-parse", "HEAD"])?.trim().to_owned();
    Sandbox::write(&top.join("file.txt"), "synthetic two\n")?;
    sandbox.commit(&top, "second")?;
    let second = sandbox.run(&top, &["rev-parse", "HEAD"])?.trim().to_owned();
    let git = sandbox.git()?;
    assert_eq!(
        repo::first_parent(&git, &top, &second)?,
        Some(first.clone())
    );
    assert_eq!(repo::first_parent(&git, &top, &first)?, None);
    assert!(matches!(
        repo::first_parent(&git, &top, "no-such-commit"),
        Err(GitError::Failed { .. })
    ));
    Ok(())
}
