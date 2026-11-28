//! Stage and unstage by record, in real repositories.

use std::path::Path;
use std::time::Duration;

use theos_studio::changes::{Changes, RecordChange, changes};
use theos_studio::git_write::{stage_records, unstage_records};
use theos_studio::lock::LockError;
use theos_studio::{Rename, StudioError, Workspace};
use theos_vcs::status::{Change, Conflict, Entry};
use theos_vcs::{Cancel, Git};

use crate::common::{Error, HelperProcess, TestResult, put, spawn_helper};
use crate::helper::HelperTask;
use crate::repo::Repo;

const T900: &str = "lore/truths/T-900.md";
const T901: &str = "lore/truths/T-901.md";

/// The changes of the project at `place` below the top of `repo`.
fn changes_of(repo: &Repo, place: &str) -> Result<Changes, Error> {
    let workspace = Workspace::open(&repo.top().join(place))?;
    Ok(changes(&workspace, &repo.git()?, &Cancel::new())?)
}

/// The change of the record `id` in `changes`.
fn record<'a>(changes: &'a Changes, id: &str) -> Result<&'a RecordChange, Error> {
    changes
        .records
        .iter()
        .find(|record| record.id == id)
        .ok_or_else(|| format!("no change of {id}: {changes:?}").into())
}

/// Appends a line to the body of `path` below `folder`.
fn edit(folder: &Path, path: &str) -> Result<(), Error> {
    let text = std::fs::read_to_string(folder.join(path))?;
    put(folder, path, format!("{text}\nEdited.\n").as_bytes())?;
    Ok(())
}

// AC: CE-22.1
#[test]
fn a_renamed_record_stages_as_one_rename_and_unstages_back() -> TestResult {
    let repo = Repo::committed("")?;
    let git = repo.git()?;
    let mut workspace = Workspace::open(repo.top())?;
    workspace.run(&Rename {
        id: "T-901".to_owned(),
        to: "T-904".to_owned(),
    })?;
    let found = changes(&workspace, &git, &Cancel::new())?;
    let all: Vec<&RecordChange> = found.records.iter().collect();
    assert!(all.iter().any(|r| r.id == "T-904"), "{found:?}");

    stage_records(&mut workspace, &git, &all, &[])?;
    let staged = changes(&workspace, &git, &Cancel::new())?;
    let new = record(&staged, "T-904")?;
    assert_eq!(new.staged, Change::Renamed, "{staged:?}");
    assert_eq!(new.unstaged, Change::Unchanged);
    assert_eq!(new.repo_from.as_deref(), Some(T901.as_bytes()));
    assert_eq!(staged.records.len(), 1, "{staged:?}");

    let all: Vec<&RecordChange> = staged.records.iter().collect();
    unstage_records(&mut workspace, &git, &all, &[])?;
    let back = changes(&workspace, &git, &Cancel::new())?;
    assert!(
        back.records.iter().all(|r| r.staged == Change::Unchanged),
        "{back:?}"
    );
    assert_eq!(back.records.len(), 1, "{back:?}");
    Ok(())
}

#[test]
fn a_conflicted_record_is_refused_and_nothing_is_staged() -> TestResult {
    let repo = Repo::committed("")?;
    let git = repo.git()?;
    edit(repo.top(), T901)?;
    let mut workspace = Workspace::open(repo.top())?;
    let found = changes(&workspace, &git, &Cancel::new())?;
    let edited = record(&found, "T-901")?.clone();
    // A real conflict is built by P46's tests; here the record only has to say it is one.
    let mut conflicted = edited.clone();
    conflicted.id = "T-900".to_owned();
    conflicted.conflict = Some(Conflict::BothModified);
    let result = stage_records(&mut workspace, &git, &[&edited, &conflicted], &[]);
    assert!(
        matches!(&result, Err(StudioError::Conflicted { id }) if id == "T-900"),
        "{result:?}"
    );
    let after = changes_of(&repo, "")?;
    assert_eq!(record(&after, "T-901")?.staged, Change::Unchanged);
    Ok(())
}

#[test]
fn a_file_at_the_top_above_the_project_is_staged() -> TestResult {
    let repo = Repo::committed("content")?;
    let git = repo.git()?;
    put(repo.top(), "README", b"one\n")?;
    repo.commit("readme")?;
    put(repo.top(), "README", b"two\n")?;
    let mut workspace = Workspace::open(&repo.top().join("content"))?;
    let found = changes(&workspace, &git, &Cancel::new())?;
    let files: Vec<&Entry> = found.files.iter().collect();
    assert_eq!(files.len(), 1, "{found:?}");
    stage_records(&mut workspace, &git, &[], &files)?;
    let after = changes(&workspace, &git, &Cancel::new())?;
    assert!(
        matches!(
            after.files.as_slice(),
            [Entry::Changed { path, staged: Change::Modified, unstaged: Change::Unchanged }]
                if path == b"README"
        ),
        "{after:?}"
    );
    unstage_records(&mut workspace, &git, &[], &files)?;
    let back = changes(&workspace, &git, &Cancel::new())?;
    assert!(
        matches!(
            back.files.as_slice(),
            [Entry::Changed {
                staged: Change::Unchanged,
                unstaged: Change::Modified,
                ..
            }]
        ),
        "{back:?}"
    );
    Ok(())
}

/// A record committed as `lore/truths/T-900.md` and edited on disk stages under git's spelling,
/// also when the folder is spelled differently on disk.
fn stage_edited_record(folder_on_disk: Option<&str>) -> TestResult {
    let repo = Repo::committed("")?;
    let git = repo.git()?;
    let mut path = T900.to_owned();
    if let Some(folder) = folder_on_disk {
        std::fs::rename(repo.top().join("lore"), repo.top().join(folder))?;
        path = format!("{folder}/truths/T-900.md");
    }
    edit(repo.top(), &path)?;
    let mut workspace = Workspace::open(repo.top())?;
    let found = changes(&workspace, &git, &Cancel::new())?;
    let edited = record(&found, "T-900")?;
    assert_eq!(edited.repo_path, T900.as_bytes(), "git's spelling");
    stage_records(&mut workspace, &git, &[edited], &[])?;
    let after = changes(&workspace, &git, &Cancel::new())?;
    let staged = record(&after, "T-900")?;
    assert_eq!(staged.staged, Change::Modified, "{after:?}");
    assert_eq!(staged.unstaged, Change::Unchanged);
    assert_eq!(repo.run(&["ls-files", "--", T900])?.trim(), T900);
    Ok(())
}

// AC: CE-22.1
#[test]
fn staging_uses_git_spelling() -> TestResult {
    stage_edited_record(None)
}

// The folder is spelled `Lore` on disk and `lore` in the index: only a case-insensitive file
// system (Windows, macOS by default) lets the same file be both.
#[cfg(any(windows, target_os = "macos"))]
#[test]
fn staging_uses_git_spelling_with_the_folder_spelled_differently_on_disk() -> TestResult {
    stage_edited_record(Some("Lore"))
}

/// Starts a helper that holds the lock of `root` and waits until it has it.
fn hold(root: &Path, scratch: &Path) -> Result<HelperProcess, Error> {
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
fn a_held_lock_is_busy_after_the_wait_and_nothing_is_staged() -> TestResult {
    let repo = Repo::committed("")?;
    let git: Git = repo.git()?;
    edit(repo.top(), T900)?;
    let mut workspace = Workspace::open(repo.top())?;
    workspace.set_lock_wait(Duration::from_millis(200));
    let found = changes(&workspace, &git, &Cancel::new())?;
    let records: Vec<&RecordChange> = found.records.iter().collect();

    let scratch = tempfile::tempdir()?;
    let mut child = hold(repo.top(), scratch.path())?;
    let result = stage_records(&mut workspace, &git, &records, &[]);
    assert!(
        matches!(&result, Err(StudioError::Lock(LockError::Busy { .. }))),
        "{result:?}"
    );
    let busy = changes_of(&repo, "")?;
    assert_eq!(record(&busy, "T-900")?.staged, Change::Unchanged);

    std::fs::write(scratch.path().join("release"), "")?;
    child.wait()?;
    stage_records(&mut workspace, &git, &records, &[])?;
    let done = changes_of(&repo, "")?;
    assert_eq!(record(&done, "T-900")?.staged, Change::Modified);
    Ok(())
}
