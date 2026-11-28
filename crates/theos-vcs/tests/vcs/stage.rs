//! `stage` and `unstage` by paths, and `repo::head`.

use std::path::Path;

use theos_vcs::status::{self, Change, Entry};
use theos_vcs::{Cancel, repo, stage};

use crate::common::Sandbox;

type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;

fn entries(sandbox: &Sandbox, top: &Path) -> Result<Vec<Entry>> {
    Ok(status::status(&sandbox.git()?, top, &Cancel::new())?.entries)
}

fn untracked(path: &str) -> Entry {
    Entry::Untracked {
        path: path.as_bytes().to_vec(),
    }
}

fn staged_added(path: &str) -> Entry {
    Entry::Changed {
        path: path.as_bytes().to_vec(),
        staged: Change::Added,
        unstaged: Change::Unchanged,
    }
}

const BODY: &str = "line one\nline two\nline three\nline four\nline five\nline six\n";

#[test]
fn a_rename_on_disk_stages_as_one_renamed_entry() -> Result {
    let sandbox = Sandbox::new()?;
    let top = sandbox.init("repo")?;
    Sandbox::write(&top.join("i.md"), BODY)?;
    sandbox.commit(&top, "first")?;
    std::fs::rename(top.join("i.md"), top.join("i2.md"))?;

    stage::stage(&sandbox.git()?, &top, &[b"i.md", b"i2.md"])?;

    let found = entries(&sandbox, &top)?;
    assert_eq!(found.len(), 1, "{found:?}");
    assert!(
        matches!(
            &found[0],
            Entry::Renamed {
                staged: Change::Renamed,
                ..
            }
        ),
        "{found:?}"
    );
    Ok(())
}

#[test]
fn unstaging_before_the_first_commit_leaves_the_file_untracked() -> Result {
    let sandbox = Sandbox::new()?;
    let top = sandbox.init("repo")?;
    Sandbox::write(&top.join("a.md"), BODY)?;
    let git = sandbox.git()?;
    assert_eq!(repo::head(&git, &top)?, None);

    stage::stage(&git, &top, &[b"a.md"])?;
    assert_eq!(entries(&sandbox, &top)?, [staged_added("a.md")]);
    stage::unstage(&git, &top, &[b"a.md"])?;

    assert_eq!(entries(&sandbox, &top)?, [untracked("a.md")]);
    Ok(())
}

#[test]
fn unstaging_a_staged_file_edited_since_keeps_the_edit_before_the_first_commit() -> Result {
    let sandbox = Sandbox::new()?;
    let top = sandbox.init("repo")?;
    Sandbox::write(&top.join("f.md"), "version A\n")?;
    let git = sandbox.git()?;
    stage::stage(&git, &top, &[b"f.md"])?;
    Sandbox::write(&top.join("f.md"), "version B\n")?;

    stage::unstage(&git, &top, &[b"f.md"])?;

    assert_eq!(entries(&sandbox, &top)?, [untracked("f.md")]);
    assert_eq!(std::fs::read_to_string(top.join("f.md"))?, "version B\n");
    Ok(())
}

#[test]
fn unstaging_after_a_commit_keeps_the_edit_unstaged() -> Result {
    let sandbox = Sandbox::new()?;
    let top = sandbox.init("repo")?;
    Sandbox::write(&top.join("a.md"), BODY)?;
    sandbox.commit(&top, "first")?;
    Sandbox::write(&top.join("a.md"), "changed\n")?;
    let git = sandbox.git()?;

    stage::stage(&git, &top, &[b"a.md"])?;
    stage::unstage(&git, &top, &[b"a.md"])?;

    assert_eq!(
        entries(&sandbox, &top)?,
        [Entry::Changed {
            path: b"a.md".to_vec(),
            staged: Change::Unchanged,
            unstaged: Change::Modified,
        }]
    );
    Ok(())
}

#[test]
fn head_is_the_commit_id_after_the_first_commit() -> Result {
    let sandbox = Sandbox::new()?;
    let top = sandbox.init("repo")?;
    Sandbox::write(&top.join("a.md"), BODY)?;
    sandbox.commit(&top, "first")?;

    let head = repo::head(&sandbox.git()?, &top)?.ok_or("no head")?;

    assert_eq!(
        head.to_string(),
        sandbox.run(&top, &["rev-parse", "HEAD"])?.trim()
    );
    Ok(())
}

#[cfg(not(windows))]
fn literal_names() -> Vec<&'static str> {
    // Windows forbids `:` and `*` in file names, so `x:y.md` and `*.md` only run elsewhere.
    vec!["a b.md", "*.md", "x:y.md", "-lead.md", "ü.md"]
}

#[cfg(windows)]
fn literal_names() -> Vec<&'static str> {
    vec!["a b.md", "-lead.md", "ü.md"]
}

#[test]
fn paths_are_staged_literally_and_nothing_else_is() -> Result {
    let sandbox = Sandbox::new()?;
    let top = sandbox.init("repo")?;
    let names = literal_names();
    for name in &names {
        Sandbox::write(&top.join(name), BODY)?;
    }
    Sandbox::write(&top.join("ab.md"), BODY)?;
    let paths: Vec<&[u8]> = names.iter().map(|name| name.as_bytes()).collect();

    stage::stage(&sandbox.git()?, &top, &paths)?;

    let found = entries(&sandbox, &top)?;
    assert!(found.contains(&untracked("ab.md")), "{found:?}");
    for name in &names {
        assert!(found.contains(&staged_added(name)), "{name}: {found:?}");
    }
    assert_eq!(found.len(), names.len() + 1, "{found:?}");
    Ok(())
}

#[test]
fn three_thousand_paths_stage_in_one_call() -> Result {
    let sandbox = Sandbox::new()?;
    let top = sandbox.init("repo")?;
    let names: Vec<String> = (0..3000).map(|n| format!("file-{n:012}.md")).collect();
    assert!(names.iter().all(|name| name.len() == 20));
    for name in &names {
        Sandbox::write(&top.join(name), "x\n")?;
    }
    let paths: Vec<&[u8]> = names.iter().map(String::as_bytes).collect();

    stage::stage(&sandbox.git()?, &top, &paths)?;

    let staged = sandbox.run(&top, &["diff", "--cached", "--name-only"])?;
    assert_eq!(staged.lines().count(), 3000);
    Ok(())
}

#[test]
fn an_empty_list_runs_no_git() -> Result {
    let sandbox = Sandbox::new()?;
    let git = sandbox.git()?;
    stage::stage(&git, sandbox.path(), &[])?;
    stage::unstage(&git, sandbox.path(), &[])?;
    Ok(())
}
