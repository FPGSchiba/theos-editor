//! `merge`, `merge_state`, `conclude`, `abort_merge`, `changed_since` and `housekeeping` against
//! local bare repositories.

use std::path::{Path, PathBuf};

use theos_vcs::housekeeping::housekeeping;
use theos_vcs::merge::{
    Merged, abort_merge, changed_since, conclude, merge, merge_base, merge_state,
};
use theos_vcs::{Cancel, repo};

use crate::common::Sandbox;

type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;

/// A synthetic identity in the repository's own configuration (`conclude` commits).
fn identify(sandbox: &Sandbox, top: &Path) -> Result {
    sandbox.run(top, &["config", "user.name", "Synthetic"])?;
    sandbox.run(top, &["config", "user.email", "synthetic@example.invalid"])?;
    Ok(())
}

/// Two clones `a` and `b` of one bare `origin` holding `a.md` with three lines. Returns
/// `(a, b)`.
fn two_clones(sandbox: &Sandbox) -> Result<(PathBuf, PathBuf)> {
    let seed = sandbox.init("seed")?;
    identify(sandbox, &seed)?;
    Sandbox::write(&seed.join("a.md"), "one\ntwo\nthree\n")?;
    sandbox.commit(&seed, "first")?;
    let bare = sandbox.path().join("origin.git");
    let bare_text = bare.to_str().ok_or("path")?;
    let seed_text = seed.to_str().ok_or("path")?;
    sandbox.run(
        sandbox.path(),
        &["clone", "-q", "--bare", seed_text, bare_text],
    )?;
    let mut clones = Vec::new();
    for name in ["a", "b"] {
        let folder = sandbox.path().join(name);
        let text = folder.to_str().ok_or("path")?;
        sandbox.run(sandbox.path(), &["clone", "-q", bare_text, text])?;
        identify(sandbox, &folder)?;
        clones.push(folder);
    }
    let b = clones.pop().ok_or("b")?;
    let a = clones.pop().ok_or("a")?;
    Ok((a, b))
}

/// Commits `text` as `file` in `top` and pushes it.
fn commit_and_push(sandbox: &Sandbox, top: &Path, file: &str, text: &str) -> Result {
    Sandbox::write(&top.join(file), text)?;
    sandbox.commit(top, "change")?;
    sandbox.run(top, &["push", "-q", "origin", "main"])?;
    Ok(())
}

/// `a` committed locally and `b` pushed, on different files.
fn diverged_clean(sandbox: &Sandbox) -> Result<PathBuf> {
    let (a, b) = two_clones(sandbox)?;
    commit_and_push(sandbox, &b, "b.md", "remote\n")?;
    Sandbox::write(&a.join("a2.md"), "local\n")?;
    sandbox.commit(&a, "local")?;
    sandbox.run(&a, &["fetch", "-q", "origin"])?;
    Ok(a)
}

/// `a` and `b` both changed line two of `a.md`.
fn diverged_conflicting(sandbox: &Sandbox) -> Result<PathBuf> {
    let (a, b) = two_clones(sandbox)?;
    commit_and_push(sandbox, &b, "a.md", "one\nREMOTE\nthree\n")?;
    Sandbox::write(&a.join("a.md"), "one\nLOCAL\nthree\n")?;
    sandbox.commit(&a, "local")?;
    sandbox.run(&a, &["fetch", "-q", "origin"])?;
    Ok(a)
}

// AC: CE-22.3
#[test]
fn a_clone_behind_its_remote_fast_forwards() -> Result {
    let sandbox = Sandbox::new()?;
    let (a, b) = two_clones(&sandbox)?;
    commit_and_push(&sandbox, &b, "b.md", "remote\n")?;
    sandbox.run(&a, &["fetch", "-q", "origin"])?;
    let git = sandbox.git()?;
    let remote = repo::head(&git, &b)?.ok_or("head")?;

    let merged = merge(&git, &a, "origin/main")?;

    assert_eq!(merged, Merged::FastForward { to: remote.clone() });
    assert_eq!(repo::head(&git, &a)?, Some(remote));
    assert_eq!(merge_state(&git, &a)?, None);
    assert_eq!(merge(&git, &a, "origin/main")?, Merged::UpToDate);
    Ok(())
}

// AC: CE-22.3
#[test]
fn the_users_merge_settings_do_not_change_the_merge() -> Result {
    let sandbox = Sandbox::new()?;
    let (a, b) = two_clones(&sandbox)?;
    commit_and_push(&sandbox, &b, "b.md", "remote\n")?;
    sandbox.run(&a, &["fetch", "-q", "origin"])?;
    sandbox.run(&a, &["config", "merge.ff", "false"])?;
    sandbox.run(&a, &["config", "merge.autoStash", "true"])?;
    let git = sandbox.git()?;

    let merged = merge(&git, &a, "origin/main")?;

    assert!(matches!(merged, Merged::FastForward { .. }), "{merged:?}");
    assert_eq!(sandbox.run(&a, &["stash", "list"])?, "");
    Ok(())
}

// AC: CE-22.3
#[test]
fn a_clean_divergence_is_staged_and_conclude_makes_a_merge_commit() -> Result {
    let sandbox = Sandbox::new()?;
    let a = diverged_clean(&sandbox)?;
    let git = sandbox.git()?;
    let upstream = repo::head(&git, &a)?.ok_or("head")?;
    let remote = sandbox.run(&a, &["rev-parse", "origin/main"])?;

    assert_eq!(merge(&git, &a, "origin/main")?, Merged::Staged);

    let state = merge_state(&git, &a)?.ok_or("no merge state")?;
    assert_eq!(state.head.0, remote.trim());
    assert!(state.message.starts_with("Merge"), "{}", state.message);
    assert_eq!(repo::head(&git, &a)?, Some(upstream));

    let done = conclude(&git, &a)?;
    let parents = sandbox.run(&a, &["rev-list", "--parents", "-n", "1", &done.0])?;
    assert_eq!(parents.split_whitespace().count(), 3, "{parents}");
    assert_eq!(merge_state(&git, &a)?, None);
    Ok(())
}

// AC: CE-22.3
#[test]
fn the_same_line_changed_on_both_sides_conflicts() -> Result {
    let sandbox = Sandbox::new()?;
    let a = diverged_conflicting(&sandbox)?;
    let git = sandbox.git()?;

    assert_eq!(merge(&git, &a, "origin/main")?, Merged::Conflicts);

    assert_ne!(sandbox.run(&a, &["ls-files", "-u"])?, "");
    assert!(merge_state(&git, &a)?.is_some());
    Ok(())
}

// AC: CE-22.3
#[test]
fn abort_restores_head_after_staged_and_after_conflicts() -> Result {
    for conflicting in [false, true] {
        let sandbox = Sandbox::new()?;
        let a = if conflicting {
            diverged_conflicting(&sandbox)?
        } else {
            diverged_clean(&sandbox)?
        };
        let git = sandbox.git()?;
        let head = repo::head(&git, &a)?;
        merge(&git, &a, "origin/main")?;

        abort_merge(&git, &a)?;

        assert_eq!(repo::head(&git, &a)?, head);
        assert_eq!(merge_state(&git, &a)?, None);
        assert!(!a.join(".git").join("MERGE_HEAD").exists());
        assert_eq!(sandbox.run(&a, &["status", "--porcelain"])?, "");
    }
    Ok(())
}

// AC: CE-22.3
#[test]
fn changed_since_lists_the_upstreams_paths() -> Result {
    let sandbox = Sandbox::new()?;
    let (a, b) = two_clones(&sandbox)?;
    commit_and_push(&sandbox, &b, "b.md", "remote\n")?;
    sandbox.run(&a, &["fetch", "-q", "origin"])?;
    let git = sandbox.git()?;

    let base = merge_base(&git, &a, "HEAD", "origin/main")?.ok_or("no base")?;
    let paths = changed_since(&git, &a, &base, "origin/main")?;

    assert_eq!(paths, vec![b"b.md".to_vec()]);
    assert!(merge_base(&git, &a, "HEAD", "-x").is_err());
    Ok(())
}

#[test]
fn housekeeping_on_a_fresh_repository_is_ok() -> Result {
    let sandbox = Sandbox::new()?;
    let top = sandbox.init("repo")?;
    Sandbox::write(&top.join("a.md"), "one\n")?;
    sandbox.commit(&top, "first")?;

    housekeeping(&sandbox.git()?, &top, &Cancel::new())?;
    Ok(())
}
