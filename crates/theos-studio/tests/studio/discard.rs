//! Discard: a record's unstaged changes go, the working tree back to the index, undoable.

use std::path::Path;

use theos_studio::changes::{Changes, RecordChange, changes};
use theos_studio::git_write::unstage_records;
use theos_studio::{CommandError, Discard, Rename, Scope, StudioError, Workspace};
use theos_vcs::status::{Change, Conflict, Entry};
use theos_vcs::{Cancel, Git};

use crate::common::{Error, TestResult, holds, put};
use crate::repo::Repo;

const T900: &str = "lore/truths/T-900.md";
const T901: &str = "lore/truths/T-901.md";
const C900: &str = "lore/clues/C-900.md";

/// The changes of the project of `workspace`.
fn changes_now(workspace: &Workspace, git: &Git) -> Result<Changes, Error> {
    Ok(changes(workspace, git, &Cancel::new())?)
}

/// The change of the record `id` in `changes`.
fn record<'a>(changes: &'a Changes, id: &str) -> Result<&'a RecordChange, Error> {
    changes
        .records
        .iter()
        .find(|record| record.id == id)
        .ok_or_else(|| format!("no change of {id}: {changes:?}").into())
}

/// Appends a line to the body of `path` below `folder` and returns the new bytes.
fn edit(folder: &Path, path: &str) -> Result<Vec<u8>, Error> {
    let mut bytes = std::fs::read(folder.join(path))?;
    bytes.extend_from_slice(b"\nEdited.\n");
    put(folder, path, &bytes)?;
    Ok(bytes)
}

/// The bytes of `path` below `folder`, `None` when it is absent.
fn read(folder: &Path, path: &str) -> Result<Option<Vec<u8>>, Error> {
    match std::fs::read(folder.join(path)) {
        Ok(bytes) => Ok(Some(bytes)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error.into()),
    }
}

/// Builds the discard of the record `id` and runs it.
fn discard_record(
    workspace: &mut Workspace,
    git: &Git,
    id: &str,
) -> Result<theos_studio::Outcome, Error> {
    let found = changes_now(workspace, git)?;
    let discard = Discard::record(workspace, git, record(&found, id)?)?;
    Ok(workspace.run(&discard)?)
}

/// The `NotDiscardable` error of `result`, which must name `git restore`.
fn not_discardable(result: Result<Discard, StudioError>, path: &str) -> TestResult {
    match result {
        Err(error @ StudioError::NotDiscardable { .. }) => {
            let StudioError::NotDiscardable { path: named, .. } = &error else {
                return Err("unreachable".into());
            };
            assert_eq!(named, path);
            assert!(error.to_string().contains("git restore"), "{error}");
            Ok(())
        }
        Err(other) => Err(format!("not NotDiscardable: {other:?}").into()),
        Ok(_) => Err("the discard was built".into()),
    }
}

#[test]
fn a_modified_record_gets_the_index_bytes_and_undo_brings_the_edit_back() -> TestResult {
    let repo = Repo::committed("")?;
    let git = repo.git()?;
    let committed = std::fs::read(repo.top().join(T900))?;
    let edited = edit(repo.top(), T900)?;
    let mut workspace = Workspace::open(repo.top())?;

    let found = changes_now(&workspace, &git)?;
    let discard = Discard::record(&workspace, &git, record(&found, "T-900")?)?;
    assert_eq!(
        theos_studio::Command::label(&discard),
        format!("discard {T900}")
    );
    let outcome = workspace.run(&discard)?;
    assert_eq!(outcome.changed, [T900]);
    assert_eq!(read(repo.top(), T900)?, Some(committed.clone()));
    assert_eq!(changes_now(&workspace, &git)?.records, []);

    workspace.undo(Scope::Newest)?;
    assert_eq!(read(repo.top(), T900)?, Some(edited));
    workspace.redo(Scope::Newest)?;
    assert_eq!(read(repo.top(), T900)?, Some(committed));
    Ok(())
}

#[test]
fn with_autocrlf_the_discard_writes_the_checkout_bytes() -> TestResult {
    let repo = Repo::committed("")?;
    repo.run(&["config", "core.autocrlf", "true"])?;
    let git = repo.git()?;
    let committed = std::fs::read(repo.top().join(T900))?;
    // Check the file out again under the setting, as a clone made with it would have it: the
    // index then records the CRLF file. (A file checked out before the setting keeps its LF size
    // in the index, and git's status shows it modified until something refreshes the index.)
    std::fs::remove_file(repo.top().join(T900))?;
    repo.run(&["checkout", "--", T900])?;
    edit(repo.top(), T900)?;
    let mut workspace = Workspace::open(repo.top())?;

    discard_record(&mut workspace, &git, "T-900")?;
    let crlf = String::from_utf8(committed)?.replace('\n', "\r\n");
    assert_eq!(read(repo.top(), T900)?, Some(crlf.into_bytes()));
    let after = changes_now(&workspace, &git)?;
    assert_eq!(after.records, [], "{after:?}");
    Ok(())
}

/// Spec §5.13: an `eol=crlf` attribute gives the checkout bytes CRLF too.
#[test]
fn with_an_eol_crlf_attribute_the_discard_writes_the_checkout_bytes() -> TestResult {
    let repo = Repo::committed("")?;
    put(repo.top(), ".gitattributes", b"*.md eol=crlf\n")?;
    repo.commit("attributes")?;
    let git = repo.git()?;
    let committed = std::fs::read(repo.top().join(T900))?;
    // Check the file out again under the attribute, as a clone has it.
    std::fs::remove_file(repo.top().join(T900))?;
    repo.run(&["checkout", "--", T900])?;
    edit(repo.top(), T900)?;
    let mut workspace = Workspace::open(repo.top())?;

    discard_record(&mut workspace, &git, "T-900")?;
    let crlf = String::from_utf8(committed)?.replace('\n', "\r\n");
    assert_eq!(read(repo.top(), T900)?, Some(crlf.into_bytes()));
    let after = changes_now(&workspace, &git)?;
    assert_eq!(after.records, [], "{after:?}");
    Ok(())
}

/// A record with staged and unstaged changes (`MM`): the unstaged edit goes, the staged one stays.
#[test]
fn a_record_with_staged_and_unstaged_changes_keeps_the_staged_ones() -> TestResult {
    let repo = Repo::committed("")?;
    let git = repo.git()?;
    let staged = edit(repo.top(), T900)?;
    repo.run(&["add", "--", T900])?;
    edit(repo.top(), T900)?;
    let mut workspace = Workspace::open(repo.top())?;
    let before = changes_now(&workspace, &git)?;
    let change = record(&before, "T-900")?;
    assert_eq!(
        (change.staged, change.unstaged),
        (Change::Modified, Change::Modified)
    );

    discard_record(&mut workspace, &git, "T-900")?;
    assert_eq!(read(repo.top(), T900)?, Some(staged));
    let after = changes_now(&workspace, &git)?;
    let change = record(&after, "T-900")?;
    assert_eq!(change.staged, Change::Modified, "{after:?}");
    assert_eq!(change.unstaged, Change::Unchanged, "{after:?}");
    Ok(())
}

/// A file of the project in a subfolder of the repository is labelled by its project path, as a
/// record is.
#[test]
fn a_file_is_labelled_by_its_project_path() -> TestResult {
    let repo = Repo::committed("content")?;
    let git = repo.git()?;
    put(repo.top(), "content/notes.txt", b"one\n")?;
    repo.commit("notes")?;
    put(repo.top(), "content/notes.txt", b"two\n")?;
    let mut workspace = Workspace::open(&repo.top().join("content"))?;
    let found = changes_now(&workspace, &git)?;
    let [file] = found.files.as_slice() else {
        return Err(format!("{found:?}").into());
    };
    let discard = Discard::file(&workspace, &git, file)?;
    assert_eq!(theos_studio::Command::label(&discard), "discard notes.txt");
    workspace.run(&discard)?;
    assert_eq!(
        read(repo.top(), "content/notes.txt")?.as_deref(),
        Some(&b"one\n"[..])
    );
    Ok(())
}

#[test]
fn an_unstaged_deletion_is_created_again() -> TestResult {
    let repo = Repo::committed("")?;
    let git = repo.git()?;
    let committed = std::fs::read(repo.top().join(T900))?;
    std::fs::remove_file(repo.top().join(T900))?;
    let mut workspace = Workspace::open(repo.top())?;

    let found = changes_now(&workspace, &git)?;
    assert_eq!(record(&found, "T-900")?.unstaged, Change::Deleted);
    discard_record(&mut workspace, &git, "T-900")?;
    assert_eq!(read(repo.top(), T900)?, Some(committed));
    workspace.undo(Scope::Newest)?;
    assert_eq!(read(repo.top(), T900)?, None);
    Ok(())
}

/// The record T-901 renamed to T-904 on disk by the Rename command, unstaged; with
/// `intent_to_add` the user also ran `git add -N` on the new path. Returns the repo, the workspace
/// and the bytes of the old and the new file.
fn renamed_on_disk(intent_to_add: bool) -> Result<(Repo, Workspace, Vec<u8>, Vec<u8>), Error> {
    let repo = Repo::committed("")?;
    let committed = std::fs::read(repo.top().join(T901))?;
    let mut workspace = Workspace::open(repo.top())?;
    workspace.run(&Rename {
        id: "T-901".to_owned(),
        to: "T-904".to_owned(),
    })?;
    let renamed = std::fs::read(repo.top().join(T904))?;
    if intent_to_add {
        repo.run(&["add", "-N", "--", T904])?;
    }
    Ok((repo, workspace, committed, renamed))
}

const T904: &str = "lore/truths/T-904.md";

/// Discards the paired unstaged rename to T-904 and checks the files, then undoes it.
fn discard_rename_and_undo(
    repo: &Repo,
    workspace: &mut Workspace,
    committed: &[u8],
    renamed: &[u8],
) -> Result<Changes, Error> {
    let git = repo.git()?;
    let found = changes_now(workspace, &git)?;
    let change = record(&found, "T-904")?;
    assert_eq!(change.unstaged, Change::Renamed, "{found:?}");
    assert_eq!(change.from.as_deref(), Some(T901));
    assert_eq!(found.records.len(), 1, "{found:?}");

    let discard = Discard::record(workspace, &git, change)?;
    workspace.run(&discard)?;
    assert_eq!(read(repo.top(), T901)?.as_deref(), Some(committed));
    assert_eq!(read(repo.top(), T904)?, None);
    let after = changes_now(workspace, &git)?;

    workspace.undo(Scope::Newest)?;
    assert_eq!(read(repo.top(), T901)?, None);
    assert_eq!(read(repo.top(), T904)?.as_deref(), Some(renamed));
    Ok(after)
}

/// The usual case: status pairs the rename on a temporary index, the real index has no entry
/// for the new path, so the discard leaves nothing behind.
#[test]
fn a_paired_unstaged_rename_goes_back_and_undo_brings_it_again() -> TestResult {
    let (repo, mut workspace, committed, renamed) = renamed_on_disk(false)?;
    assert_eq!(repo.run(&["ls-files", "--", T904])?, "");
    let after = discard_rename_and_undo(&repo, &mut workspace, &committed, &renamed)?;
    assert_eq!(after.records, [], "{after:?}");
    assert_eq!(after.files, [], "{after:?}");
    Ok(())
}

/// The user ran `git add -N` on the new path: the real index holds an intent-to-add entry, which
/// a discard does not touch (it writes the working tree only). That entry remains as a file
/// deleted on disk, no longer a record, and Unstage clears it.
#[test]
fn a_rename_with_the_new_path_added_with_intent_leaves_that_entry() -> TestResult {
    let (repo, mut workspace, committed, renamed) = renamed_on_disk(true)?;
    let after = discard_rename_and_undo(&repo, &mut workspace, &committed, &renamed)?;
    assert_eq!(after.records, [], "{after:?}");
    let left = Entry::Changed {
        path: T904.as_bytes().to_vec(),
        staged: Change::Unchanged,
        unstaged: Change::Deleted,
    };
    assert_eq!(after.files, std::slice::from_ref(&left), "{after:?}");

    // Discard it again (undone above), then Unstage takes the entry out of the index.
    let git = repo.git()?;
    workspace.redo(Scope::Newest)?;
    unstage_records(&mut workspace, &git, &[], &[&left])?;
    let clean = changes_now(&workspace, &git)?;
    assert_eq!(clean.records, [], "{clean:?}");
    assert_eq!(clean.files, [], "{clean:?}");
    assert_eq!(repo.run(&["ls-files", "--", T904])?, "");
    Ok(())
}

#[test]
fn an_untracked_record_is_removed_and_undo_creates_it_again() -> TestResult {
    let repo = Repo::committed("")?;
    let git = repo.git()?;
    let path = "lore/truths/T-962.md";
    let text = std::fs::read_to_string(repo.top().join(T900))?.replace("T-900", "T-962");
    put(repo.top(), path, text.as_bytes())?;
    let mut workspace = Workspace::open(repo.top())?;

    let found = changes_now(&workspace, &git)?;
    assert!(record(&found, "T-962")?.untracked);
    discard_record(&mut workspace, &git, "T-962")?;
    assert_eq!(read(repo.top(), path)?, None);
    workspace.undo(Scope::Newest)?;
    assert_eq!(read(repo.top(), path)?, Some(text.into_bytes()));
    Ok(())
}

#[test]
fn a_record_with_only_staged_changes_plans_nothing() -> TestResult {
    let repo = Repo::committed("")?;
    let git = repo.git()?;
    let edited = edit(repo.top(), T900)?;
    repo.run(&["add", "--", T900])?;
    let mut workspace = Workspace::open(repo.top())?;

    let outcome = discard_record(&mut workspace, &git, "T-900")?;
    assert!(outcome.changed.is_empty(), "{outcome:?}");
    assert_eq!(outcome.entry, None);
    assert_eq!(read(repo.top(), T900)?, Some(edited));
    assert_eq!(
        record(&changes_now(&workspace, &git)?, "T-900")?.staged,
        Change::Modified
    );
    Ok(())
}

#[test]
fn an_index_version_with_an_error_is_restored_and_named() -> TestResult {
    let repo = Repo::committed("")?;
    let git = repo.git()?;
    let fixed = std::fs::read_to_string(repo.top().join(C900))?;
    let broken = fixed.replace("reveals: T-900", "reveals: T-999");
    put(repo.top(), C900, broken.as_bytes())?;
    repo.run(&["add", "--", C900])?;
    // The working copy fixed the link the index breaks.
    put(repo.top(), C900, fixed.as_bytes())?;
    let mut workspace = Workspace::open(repo.top())?;

    let outcome = discard_record(&mut workspace, &git, "C-900")?;
    assert_eq!(read(repo.top(), C900)?, Some(broken.into_bytes()));
    assert!(
        holds(&outcome.introduced, C900, "link/dangling"),
        "{outcome:?}"
    );
    assert!(holds(&outcome.findings, C900, "link/dangling"));
    Ok(())
}

#[test]
fn an_edit_after_the_discard_was_built_is_stale() -> TestResult {
    let repo = Repo::committed("")?;
    let git = repo.git()?;
    edit(repo.top(), T900)?;
    let mut workspace = Workspace::open(repo.top())?;
    let found = changes_now(&workspace, &git)?;
    let discard = Discard::record(&workspace, &git, record(&found, "T-900")?)?;
    let again = edit(repo.top(), T900)?;

    let result = workspace.run(&discard);
    assert!(
        matches!(&result, Err(CommandError::Stale { path }) if path == T900),
        "{result:?}"
    );
    assert_eq!(read(repo.top(), T900)?, Some(again));
    Ok(())
}

#[test]
fn a_conflicted_record_is_refused() -> TestResult {
    let repo = Repo::committed("")?;
    let git = repo.git()?;
    edit(repo.top(), T900)?;
    let workspace = Workspace::open(repo.top())?;
    let found = changes_now(&workspace, &git)?;
    // A real conflict is built by P46's tests; here the record only has to say it is one.
    let mut conflicted = record(&found, "T-900")?.clone();
    conflicted.conflict = Some(Conflict::BothModified);
    let result = Discard::record(&workspace, &git, &conflicted);
    assert!(
        matches!(&result, Err(StudioError::Conflicted { id }) if id == "T-900"),
        "{result:?}"
    );
    let file = Entry::Conflicted {
        path: T900.as_bytes().to_vec(),
        conflict: Conflict::BothModified,
    };
    let result = Discard::file(&workspace, &git, &file);
    assert!(
        matches!(&result, Err(StudioError::Conflicted { id }) if id == T900),
        "{result:?}"
    );
    Ok(())
}

#[test]
fn a_file_outside_the_project_is_not_discardable() -> TestResult {
    let repo = Repo::committed("content")?;
    let git = repo.git()?;
    put(repo.top(), "README", b"one\n")?;
    repo.commit("readme")?;
    put(repo.top(), "README", b"two\n")?;
    let workspace = Workspace::open(&repo.top().join("content"))?;
    let found = changes_now(&workspace, &git)?;
    let [file] = found.files.as_slice() else {
        return Err(format!("{found:?}").into());
    };
    not_discardable(Discard::file(&workspace, &git, file), "README")?;
    assert_eq!(read(repo.top(), "README")?.as_deref(), Some(&b"two\n"[..]));
    Ok(())
}

/// Review Focus 2: the old path of a rename from outside the project folder is outside it too.
#[test]
fn a_rename_from_outside_the_project_is_not_discardable() -> TestResult {
    let repo = Repo::committed("content")?;
    let git = repo.git()?;
    edit(&repo.top().join("content"), T900)?;
    let workspace = Workspace::open(&repo.top().join("content"))?;
    let found = changes_now(&workspace, &git)?;
    let mut renamed = record(&found, "T-900")?.clone();
    renamed.unstaged = Change::Renamed;
    renamed.from = None;
    renamed.repo_from = Some(b"elsewhere/T-900.md".to_vec());
    not_discardable(
        Discard::record(&workspace, &git, &renamed),
        "elsewhere/T-900.md",
    )
}

#[test]
fn a_gitignore_is_not_discardable() -> TestResult {
    let repo = Repo::committed("")?;
    let git = repo.git()?;
    put(repo.top(), "lore/.gitignore", b"*.tmp\n")?;
    repo.commit("ignore")?;
    put(repo.top(), "lore/.gitignore", b"*.bak\n")?;
    let workspace = Workspace::open(repo.top())?;
    let found = changes_now(&workspace, &git)?;
    let file = found
        .files
        .iter()
        .find(|entry| matches!(entry, Entry::Changed { path, .. } if path == b"lore/.gitignore"))
        .ok_or_else(|| format!("{found:?}"))?;
    not_discardable(Discard::file(&workspace, &git, file), "lore/.gitignore")?;
    assert_eq!(
        read(repo.top(), "lore/.gitignore")?.as_deref(),
        Some(&b"*.bak\n"[..])
    );
    Ok(())
}

#[test]
fn a_link_in_the_index_is_not_discardable() -> TestResult {
    let repo = Repo::committed("")?;
    let git = repo.git()?;
    let target = tempfile::NamedTempFile::new()?;
    std::fs::write(target.path(), "T-900.md")?;
    let target_path = target.path().to_str().ok_or("temp path")?;
    let blob = repo.run(&["hash-object", "-w", "--", target_path])?;
    let link = "notes/link.txt";
    let info = format!("120000,{},{link}", blob.trim());
    repo.run(&["update-index", "--add", "--cacheinfo", &info])?;
    let workspace = Workspace::open(repo.top())?;
    let found = changes_now(&workspace, &git)?;
    let file = found
        .files
        .iter()
        .find(|entry| matches!(entry, Entry::Changed { path, .. } if path == link.as_bytes()))
        .ok_or_else(|| format!("{found:?}"))?;
    not_discardable(Discard::file(&workspace, &git, file), link)?;
    assert_eq!(read(repo.top(), link)?, None);
    Ok(())
}
