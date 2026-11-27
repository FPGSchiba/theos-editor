//! `changes`: git's status grouped by record, in real repositories.

use std::path::Path;

use theos_studio::Workspace;
use theos_studio::changes::{Changes, RecordChange, changes};
use theos_vcs::Cancel;
use theos_vcs::status::{Change, Entry};

use crate::common::{Error, Files, TestResult, files, put};
use crate::repo::Repo;

const T900: &str = "lore/truths/T-900.md";
const T901: &str = "lore/truths/T-901.md";
const T902: &str = "lore/truths/T-902.md";

/// Opens the project at `place` below the top of `repo` and lists its changes.
fn changes_of(repo: &Repo, place: &str) -> Result<Changes, Error> {
    let workspace = Workspace::open(&repo.top().join(place))?;
    Ok(changes(&workspace, &repo.git()?, &Cancel::new())?)
}

/// The text of `path` below `folder`, with `from` replaced by `to`.
fn edited(folder: &Path, path: &str, from: &str, to: &str) -> Result<String, Error> {
    let text = std::fs::read_to_string(folder.join(path))?;
    assert!(text.contains(from), "{path} has no {from:?}");
    Ok(text.replace(from, to))
}

/// The change of the record `id` in `changes`.
fn record<'a>(changes: &'a Changes, id: &str) -> Result<&'a RecordChange, Error> {
    changes
        .records
        .iter()
        .find(|record| record.id == id)
        .ok_or_else(|| format!("no change of {id}: {changes:?}").into())
}

/// The repository paths of the entries in `changes.files`.
fn file_paths(changes: &Changes) -> Vec<Vec<u8>> {
    changes
        .files
        .iter()
        .map(|entry| match entry {
            Entry::Changed { path, .. }
            | Entry::Conflicted { path, .. }
            | Entry::Untracked { path } => path.clone(),
            Entry::Renamed { to, .. } => to.clone(),
            other => format!("{other:?}").into_bytes(),
        })
        .collect()
}

/// The files of the cache folder of the project at `root`.
fn cache(root: &Path) -> Result<Files, Error> {
    Ok(files(&root.join(".theos").join("cache"))?)
}

#[test]
fn an_edited_record_is_named_by_id_kind_and_title() -> TestResult {
    let repo = Repo::committed("")?;
    let text = edited(repo.top(), T900, "Synthetic statement A.", "Edited.")?;
    put(repo.top(), T900, text.as_bytes())?;
    let found = changes_of(&repo, "")?;
    assert_eq!(found.branch.head.as_deref(), Some("main"));
    assert_eq!(
        found.records,
        [RecordChange {
            id: "T-900".to_owned(),
            kind: Some("truth".to_owned()),
            title: Some("Synthetic truth A".to_owned()),
            path: T900.to_owned(),
            from: None,
            repo_path: T900.as_bytes().to_vec(),
            repo_from: None,
            staged: Change::Unchanged,
            unstaged: Change::Modified,
            untracked: false,
            conflict: None,
        }]
    );
    assert!(found.files.is_empty(), "{:?}", found.files);
    Ok(())
}

#[test]
fn an_unstaged_rename_with_its_id_rewritten_is_one_record() -> TestResult {
    let repo = Repo::committed("")?;
    let text = edited(repo.top(), T901, "id: T-901", "id: T-902")?;
    put(repo.top(), T902, text.as_bytes())?;
    std::fs::remove_file(repo.top().join(T901))?;
    let found = changes_of(&repo, "")?;
    assert_eq!(
        found.records,
        [RecordChange {
            id: "T-902".to_owned(),
            kind: Some("truth".to_owned()),
            title: Some("Synthetic truth B".to_owned()),
            path: T902.to_owned(),
            from: Some(T901.to_owned()),
            repo_path: T902.as_bytes().to_vec(),
            repo_from: Some(T901.as_bytes().to_vec()),
            staged: Change::Unchanged,
            unstaged: Change::Renamed,
            untracked: false,
            conflict: None,
        }]
    );
    assert!(found.files.is_empty(), "{:?}", found.files);
    Ok(())
}

#[test]
fn a_deleted_record_is_named_from_head() -> TestResult {
    let repo = Repo::committed("")?;
    repo.run(&["rm", "-q", "--", T900])?;
    let found = changes_of(&repo, "")?;
    let deleted = record(&found, "T-900")?;
    assert_eq!(deleted.kind.as_deref(), Some("truth"));
    assert_eq!(deleted.title.as_deref(), Some("Synthetic truth A"));
    assert_eq!(deleted.path, T900);
    assert_eq!(deleted.staged, Change::Deleted);
    assert_eq!(deleted.unstaged, Change::Unchanged);
    assert_eq!(found.records.len(), 1, "{:?}", found.records);
    Ok(())
}

#[test]
fn a_new_untracked_record_is_untracked() -> TestResult {
    let repo = Repo::committed("")?;
    let text = "---\n# synthetic fixture (P46), not canon\nid: T-903\nkind: truth\n\
        title: Synthetic truth C\nstatus: draft\ntarget: confirmed\n---\n\
        A wholly different synthetic statement, long enough to pair with nothing.\n";
    put(repo.top(), "lore/truths/T-903.md", text.as_bytes())?;
    let found = changes_of(&repo, "")?;
    let new = record(&found, "T-903")?;
    assert!(new.untracked);
    assert_eq!(new.kind.as_deref(), Some("truth"));
    assert_eq!(new.title.as_deref(), Some("Synthetic truth C"));
    assert_eq!(new.from, None);
    assert_eq!(found.records.len(), 1, "{:?}", found.records);
    Ok(())
}

#[test]
fn settings_and_text_without_front_matter_are_files() -> TestResult {
    let repo = Repo::committed("")?;
    let top = repo.top();
    let theos = edited(top, "theos.toml", "[project]", "# synthetic\n[project]")?;
    put(top, "theos.toml", theos.as_bytes())?;
    let settings = format!(
        "{}\n# synthetic\n",
        std::fs::read_to_string(top.join(".theos/settings.toml"))?
    );
    put(top, ".theos/settings.toml", settings.as_bytes())?;
    put(
        top,
        "lore/notes.md",
        b"Synthetic notes without front matter.\n",
    )?;
    let found = changes_of(&repo, "")?;
    assert!(found.records.is_empty(), "{:?}", found.records);
    let mut paths = file_paths(&found);
    paths.sort();
    assert_eq!(
        paths,
        [
            b".theos/settings.toml".to_vec(),
            b"lore/notes.md".to_vec(),
            b"theos.toml".to_vec(),
        ]
    );
    Ok(())
}

#[test]
fn a_project_in_a_subfolder_maps_its_paths_and_keeps_the_rest_apart() -> TestResult {
    let repo = Repo::committed("content")?;
    let project = repo.top().join("content");
    let text = edited(&project, T900, "Synthetic statement A.", "Edited.")?;
    put(&project, T900, text.as_bytes())?;
    put(repo.top(), "outside.md", b"---\nid: x\nkind: truth\n---\n")?;
    let found = changes_of(&repo, "content")?;
    let edited = record(&found, "T-900")?;
    assert_eq!(edited.path, T900);
    assert_eq!(edited.repo_path, format!("content/{T900}").into_bytes());
    assert_eq!(edited.unstaged, Change::Modified);
    assert_eq!(found.records.len(), 1, "{:?}", found.records);
    assert_eq!(
        found.files,
        [Entry::Untracked {
            path: b"outside.md".to_vec()
        }]
    );
    Ok(())
}

#[test]
fn changes_neither_sync_nor_write_the_cache() -> TestResult {
    let repo = Repo::committed("")?;
    let workspace = Workspace::open(repo.top())?;
    let text = edited(repo.top(), T900, "Synthetic statement A.", "Edited.")?;
    put(repo.top(), T900, text.as_bytes())?;
    put(repo.top(), "lore/notes.md", b"Synthetic notes.\n")?;
    let before = cache(repo.top())?;
    assert!(!before.is_empty(), "the workspace wrote no cache");
    let found = changes(&workspace, &repo.git()?, &Cancel::new())?;
    assert_eq!(record(&found, "T-900")?.unstaged, Change::Modified);
    assert_eq!(cache(repo.top())?, before);
    Ok(())
}
