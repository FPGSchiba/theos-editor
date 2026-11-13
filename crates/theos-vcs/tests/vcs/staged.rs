//! The staged snapshot: staged bytes, changes, tracked paths and the folder it is read for.

use std::path::Path;

use theos_vcs::staged::{self, Staged, StagedPath, Status};

use crate::common::Sandbox;

type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;

fn changed(path: &str, status: Status) -> StagedPath {
    StagedPath {
        path: path.as_bytes().to_vec(),
        status,
    }
}

fn tracked_paths(staged: &Staged) -> Vec<&[u8]> {
    staged.tracked.iter().map(|t| t.path.as_slice()).collect()
}

fn read(sandbox: &Sandbox, folder: &Path) -> Result<Staged> {
    Ok(staged::read(&sandbox.git()?, folder)?)
}

/// Commits `lore/a.md`, then stages a second version and leaves a third on disk.
fn staged_and_edited(sandbox: &Sandbox) -> Result<std::path::PathBuf> {
    let repo = sandbox.init("repo")?;
    let file = repo.join("lore").join("a.md");
    Sandbox::write(&file, "one\n")?;
    sandbox.commit(&repo, "first")?;
    Sandbox::write(&file, "two\n")?;
    sandbox.run(&repo, &["add", "lore/a.md"])?;
    Ok(repo)
}

#[test]
fn a_file_staged_and_edited_again_gives_its_staged_bytes() -> Result {
    let sandbox = Sandbox::new()?;
    let repo = staged_and_edited(&sandbox)?;
    Sandbox::write(&repo.join("lore").join("a.md"), "three\n")?;
    let staged = read(&sandbox, &repo)?;
    assert_eq!(staged.changed, vec![changed("lore/a.md", Status::Modified)]);
    assert_eq!(
        staged.modified,
        vec![(b"lore/a.md".to_vec(), b"two\n".to_vec())]
    );
    assert_eq!(tracked_paths(&staged), vec![b"lore/a.md".as_slice()]);
    assert_eq!(staged.tracked[0].mode, 0o100_644);
    assert!(staged.tracked[0].is_regular());
    assert_eq!(
        std::fs::canonicalize(&staged.top)?,
        std::fs::canonicalize(&repo)?
    );
    Ok(())
}

#[test]
fn a_file_deleted_on_disk_but_staged_gives_its_staged_bytes() -> Result {
    let sandbox = Sandbox::new()?;
    let repo = staged_and_edited(&sandbox)?;
    std::fs::remove_file(repo.join("lore").join("a.md"))?;
    let staged = read(&sandbox, &repo)?;
    assert_eq!(
        staged.modified,
        vec![(b"lore/a.md".to_vec(), b"two\n".to_vec())]
    );
    Ok(())
}

#[test]
fn a_file_equal_to_its_staged_version_is_not_modified() -> Result {
    let sandbox = Sandbox::new()?;
    let repo = staged_and_edited(&sandbox)?;
    let staged = read(&sandbox, &repo)?;
    assert_eq!(staged.modified.len(), 0, "{:?}", staged.modified);
    Ok(())
}

#[test]
fn a_staged_deletion_is_not_tracked() -> Result {
    let sandbox = Sandbox::new()?;
    let repo = sandbox.init("repo")?;
    Sandbox::write(&repo.join("a.md"), "one\n")?;
    sandbox.commit(&repo, "first")?;
    sandbox.run(&repo, &["rm", "-q", "--cached", "a.md"])?;
    assert!(repo.join("a.md").exists());
    let staged = read(&sandbox, &repo)?;
    assert_eq!(staged.changed, vec![changed("a.md", Status::Deleted)]);
    assert_eq!(staged.tracked.len(), 0, "{:?}", staged.tracked);
    Ok(())
}

#[test]
fn another_index_is_read_through_the_environment() -> Result {
    let sandbox = Sandbox::new()?;
    let repo = sandbox.init("repo")?;
    Sandbox::write(&repo.join("a.md"), "one\n")?;
    sandbox.commit(&repo, "first")?;
    std::fs::copy(
        repo.join(".git").join("index"),
        repo.join(".git").join("other-index"),
    )?;
    Sandbox::write(&repo.join("b.md"), "two\n")?;
    sandbox.run_env(
        &repo,
        &["add", "b.md"],
        &[("GIT_INDEX_FILE", ".git/other-index")],
    )?;
    let other = sandbox
        .git()?
        .with_env(&[("GIT_INDEX_FILE", ".git/other-index")]);
    let staged = staged::read(&other, &repo)?;
    assert_eq!(staged.changed, vec![changed("b.md", Status::Added)]);
    assert_eq!(
        tracked_paths(&staged),
        vec![b"a.md".as_slice(), b"b.md".as_slice()]
    );
    let plain = read(&sandbox, &repo)?;
    assert_eq!(plain.changed.len(), 0, "{:?}", plain.changed);
    assert_eq!(tracked_paths(&plain), vec![b"a.md".as_slice()]);
    Ok(())
}

#[test]
fn tracked_keeps_the_modes_of_links_and_submodules() -> Result {
    let sandbox = Sandbox::new()?;
    let repo = sandbox.init("repo")?;
    Sandbox::write(&repo.join("lore").join("a.md"), "one\n")?;
    sandbox.commit(&repo, "first")?;
    let loose = sandbox.path().join("blob.txt");
    Sandbox::write(&loose, "target\n")?;
    let blob = sandbox
        .run(&repo, &["hash-object", "-w", loose.to_str().ok_or("path")?])?
        .trim()
        .to_owned();
    let head = sandbox
        .run(&repo, &["rev-parse", "HEAD"])?
        .trim()
        .to_owned();
    for entry in [
        format!("120000,{blob},lore/clues/C-901.md"),
        format!("160000,{head},lore/sub"),
    ] {
        sandbox.run(&repo, &["update-index", "--add", "--cacheinfo", &entry])?;
    }
    let staged = read(&sandbox, &repo)?;
    let mode_of = |path: &str| {
        staged
            .tracked
            .iter()
            .find(|t| t.path == path.as_bytes())
            .map(|t| (t.mode, t.is_regular()))
    };
    assert_eq!(mode_of("lore/a.md"), Some((0o100_644, true)));
    assert_eq!(mode_of("lore/clues/C-901.md"), Some((0o120_000, false)));
    assert_eq!(mode_of("lore/sub"), Some((0o160_000, false)));
    // Their files are absent on disk, yet only regular entries are read.
    assert_eq!(staged.modified.len(), 0, "{:?}", staged.modified);
    Ok(())
}

#[test]
fn a_project_in_a_subfolder_reads_only_its_folder() -> Result {
    let sandbox = Sandbox::new()?;
    let repo = sandbox.init("repo")?;
    Sandbox::write(&repo.join("content").join("a.md"), "one\n")?;
    Sandbox::write(&repo.join("other").join("b.md"), "one\n")?;
    sandbox.commit(&repo, "first")?;
    for file in ["content/a.md", "other/b.md"] {
        Sandbox::write(&repo.join(file), "two\n")?;
    }
    sandbox.run(&repo, &["add", "-A"])?;
    for file in ["content/a.md", "other/b.md"] {
        Sandbox::write(&repo.join(file), "three\n")?;
    }
    let staged = read(&sandbox, &repo.join("content"))?;
    assert_eq!(
        std::fs::canonicalize(&staged.top)?,
        std::fs::canonicalize(&repo)?
    );
    assert_eq!(tracked_paths(&staged), vec![b"content/a.md".as_slice()]);
    assert_eq!(
        staged.changed,
        vec![
            changed("content/a.md", Status::Modified),
            changed("other/b.md", Status::Modified),
        ]
    );
    assert_eq!(
        staged.modified,
        vec![(b"content/a.md".to_vec(), b"two\n".to_vec())]
    );
    Ok(())
}

#[test]
fn the_first_commit_lists_every_staged_file_as_added() -> Result {
    let sandbox = Sandbox::new()?;
    let repo = sandbox.init("repo")?;
    Sandbox::write(&repo.join("a.md"), "one\n")?;
    Sandbox::write(&repo.join("lore").join("b.md"), "two\n")?;
    sandbox.run(&repo, &["add", "-A"])?;
    let staged = read(&sandbox, &repo)?;
    assert_eq!(
        staged.changed,
        vec![
            changed("a.md", Status::Added),
            changed("lore/b.md", Status::Added),
        ]
    );
    assert_eq!(
        tracked_paths(&staged),
        vec![b"a.md".as_slice(), b"lore/b.md".as_slice()]
    );
    assert_eq!(staged.modified.len(), 0, "{:?}", staged.modified);
    Ok(())
}

#[test]
fn a_place_spelled_differently_on_disk_reads_git_spelling() -> Result {
    let sandbox = Sandbox::new()?;
    let repo = sandbox.init("repo")?;
    Sandbox::write(&repo.join("content").join("a.md"), "one\n")?;
    Sandbox::write(&repo.join("contents").join("b.md"), "one\n")?;
    sandbox.commit(&repo, "first")?;
    sandbox.run(&repo, &["config", "core.ignorecase", "true"])?;
    std::fs::rename(repo.join("content"), repo.join("Content"))?;
    let staged = read(&sandbox, &repo.join("Content"))?;
    // On a case-insensitive file system git still finds `content/a.md`; on a case-sensitive one
    // it is gone from the working tree, and `modified` holds its staged bytes. Either way the
    // entry is the project's, in git's spelling.
    assert_eq!(tracked_paths(&staged), vec![b"content/a.md".as_slice()]);
    assert_eq!(staged.place, "content");
    assert!(staged.folding.ignore_case);
    assert!(
        staged
            .modified
            .iter()
            .all(|(path, bytes)| path == b"content/a.md" && bytes == b"one\n"),
        "{:?}",
        staged.modified
    );
    Ok(())
}

#[test]
fn a_place_no_entry_lies_below_is_the_folder_as_found() -> Result {
    let sandbox = Sandbox::new()?;
    let repo = sandbox.init("repo")?;
    Sandbox::write(&repo.join("other").join("a.md"), "one\n")?;
    sandbox.commit(&repo, "first")?;
    std::fs::create_dir_all(repo.join("New"))?;
    let staged = read(&sandbox, &repo.join("New"))?;
    assert_eq!(staged.place, "New");
    assert_eq!(staged.tracked.len(), 0, "{:?}", staged.tracked);
    Ok(())
}

#[test]
fn a_folder_with_glob_characters_and_umlauts_is_read_literally() -> Result {
    let sandbox = Sandbox::new()?;
    let repo = sandbox.init("repo")?;
    let own = "[draft] Inhalt ä";
    let sibling = "d Inhalt ä";
    for folder in [own, sibling] {
        std::fs::create_dir_all(repo.join(folder))?;
        Sandbox::write(&repo.join(folder).join("a.md"), "one\n")?;
    }
    sandbox.commit(&repo, "first")?;
    for folder in [own, sibling] {
        Sandbox::write(&repo.join(folder).join("a.md"), "two\n")?;
    }
    let staged = read(&sandbox, &repo.join(own))?;
    let expected = format!("{own}/a.md");
    assert_eq!(tracked_paths(&staged), vec![expected.as_bytes()]);
    // Nothing is staged, so the edit on disk is the only difference.
    assert_eq!(staged.changed.len(), 0, "{:?}", staged.changed);
    assert_eq!(staged.modified.len(), 1);
    assert_eq!(staged.modified[0].0, expected.as_bytes());
    assert_eq!(staged.modified[0].1, b"one\n");
    Ok(())
}
