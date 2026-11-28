//! Freshness (spec §5.9): `project_sync` reports the record files changed, deleted, renamed or
//! moved outside the app, each by its path, so two files with one id are never confused. A file
//! that moved keeps its id and is reported as a move, not as a deletion. A focus sync runs at most
//! once a second, so the focus events of a window that is clicked in and out do not sync again and
//! again; a Reload always syncs.

mod support;

use std::path::Path;
use std::time::Duration;

use serde_json::{Value, json};
use support::{TestResult, app, call, copy_fixture, default_app};
use tauri::WebviewWindow;
use tauri::test::MockRuntime;

/// The records of the fixture.
const RECORDS: u64 = 22;

type Window = WebviewWindow<MockRuntime>;

/// A copy of the fixture, opened: the folder, the app and its window.
fn opened() -> Result<(tempfile::TempDir, tauri::App<MockRuntime>, Window), String> {
    opened_fixture("editor-codex")
}

/// A copy of the fixture `name`, opened: the folder, the app and its window.
fn opened_fixture(
    name: &str,
) -> Result<(tempfile::TempDir, tauri::App<MockRuntime>, Window), String> {
    let temp = tempfile::tempdir().map_err(|e| e.to_string())?;
    copy_fixture(name, temp.path()).map_err(|e| e.to_string())?;
    let (app, window) = app().map_err(|e| e.to_string())?;
    call(&window, "project_open", json!({ "path": temp.path() }))
        .map_err(|error| format!("the project does not open: {error}"))?;
    Ok((temp, app, window))
}

/// A sync for `reason` (`focus` or `reload`).
fn sync_for(window: &Window, reason: &str) -> Result<Value, String> {
    call(window, "project_sync", json!({ "reason": reason }))
        .map_err(|error| format!("sync failed: {error}"))
}

/// A focus sync.
fn sync(window: &Window) -> Result<Value, String> {
    sync_for(window, "focus")
}

/// The paths of the files in the list `key` (`changed` or `removed`) of `synced`.
fn listed<'a>(synced: &'a Value, key: &str) -> Vec<&'a str> {
    synced[key]
        .as_array()
        .map(|files| {
            files
                .iter()
                .filter_map(|file| file["path"].as_str())
                .collect()
        })
        .unwrap_or_default()
}

/// Appends a line to the body of the record file `file`.
fn touch(root: &Path, file: &str) -> std::io::Result<()> {
    let path = root.join(file);
    let text = std::fs::read_to_string(&path)? + "\nA synthetic line added by the test.\n";
    std::fs::write(path, text)
}

#[test]
fn focus_sync_at_most_once_a_second() -> TestResult {
    let (temp, _app, window) = opened()?;
    let first = sync(&window)?;
    assert_eq!(first["changed"], json!([]), "{first}");
    assert_eq!(first["moved"], json!([]), "{first}");

    touch(temp.path(), "world/tower.md")?;
    let second = sync(&window)?;
    assert_eq!(
        second["changed"],
        json!([]),
        "a second sync within a second does not read the folder: {second}"
    );
    assert_eq!(second["removed"], json!([]));
    assert_eq!(second["moved"], json!([]));
    assert_eq!(
        second["project"], first["project"],
        "it answers the open view"
    );

    std::thread::sleep(Duration::from_millis(1100));
    let third = sync(&window)?;
    assert!(
        listed(&third, "changed").contains(&"world/tower.md"),
        "{third}"
    );
    Ok(())
}

#[test]
fn deleted_record_is_reported_removed() -> TestResult {
    let (temp, _app, window) = opened()?;
    std::fs::remove_file(temp.path().join("lore/clues/C-032.md"))?;
    let synced = sync(&window)?;
    assert_eq!(
        synced["removed"],
        json!([{ "id": "C-032", "path": "lore/clues/C-032.md" }]),
        "{synced}"
    );
    assert_eq!(synced["moved"], json!([]), "{synced}");
    assert_eq!(synced["project"]["records"], RECORDS - 1);
    let gone = call(&window, "record_view", json!({ "id": "C-032" }))
        .err()
        .ok_or("the deleted record still has a view")?;
    assert_eq!(gone["kind"], "not_found", "{gone}");
    Ok(())
}

#[test]
fn changed_record_is_reported_changed() -> TestResult {
    let (temp, _app, window) = opened()?;
    touch(temp.path(), "world/tower.md")?;
    let synced = sync(&window)?;
    let changed = synced["changed"].as_array().ok_or("no changed list")?;
    assert!(
        changed.contains(&json!({ "id": "tower", "path": "world/tower.md" })),
        "{synced}"
    );
    assert_eq!(synced["removed"], json!([]));
    assert_eq!(synced["project"]["records"], RECORDS);
    Ok(())
}

#[test]
fn opening_does_not_hold_back_the_first_sync() -> TestResult {
    let (temp, _app, window) = opened()?;
    touch(temp.path(), "world/tower.md")?;
    let synced = sync(&window)?;
    assert!(
        listed(&synced, "changed").contains(&"world/tower.md"),
        "the sync right after opening reads the folder: {synced}"
    );
    Ok(())
}

#[test]
fn a_sync_without_a_project_fails_and_holds_nothing_back() -> TestResult {
    let (_app, window) = default_app()?;
    for _ in 0..2 {
        let error = call(&window, "project_sync", json!({ "reason": "focus" }))
            .err()
            .ok_or("a sync without a project answered")?;
        assert_eq!(error["kind"], "no_project", "{error}");
    }
    Ok(())
}

#[test]
fn reload_always_syncs() -> TestResult {
    let (temp, _app, window) = opened()?;
    sync(&window)?;
    touch(temp.path(), "world/tower.md")?;
    let reloaded = sync_for(&window, "reload")?;
    assert!(
        listed(&reloaded, "changed").contains(&"world/tower.md"),
        "a Reload within a second of a focus sync reads the folder: {reloaded}"
    );
    Ok(())
}

#[test]
fn an_id_renamed_outside_is_removed_and_the_new_id_changed() -> TestResult {
    let (temp, _app, window) = opened()?;
    let clues = temp.path().join("lore/clues");
    let text = std::fs::read_to_string(clues.join("C-032.md"))?;
    std::fs::write(
        clues.join("C-099.md"),
        text.replace("id: C-032", "id: C-099"),
    )?;
    std::fs::remove_file(clues.join("C-032.md"))?;
    let synced = sync(&window)?;
    assert_eq!(
        listed(&synced, "removed"),
        ["lore/clues/C-032.md"],
        "{synced}"
    );
    assert!(
        listed(&synced, "changed").contains(&"lore/clues/C-099.md"),
        "{synced}"
    );
    assert_eq!(synced["moved"], json!([]), "a new id is no move: {synced}");
    assert_eq!(synced["project"]["records"], RECORDS);
    Ok(())
}

#[test]
fn a_file_moved_with_its_id_is_reported_as_a_move() -> TestResult {
    let (temp, _app, window) = opened()?;
    let clues = temp.path().join("lore/clues");
    std::fs::create_dir(clues.join("moved"))?;
    std::fs::rename(clues.join("C-032.md"), clues.join("moved/C-032.md"))?;
    let synced = sync(&window)?;
    assert_eq!(
        synced["removed"],
        json!([]),
        "a moved file is not deleted: {synced}"
    );
    assert_eq!(
        synced["moved"],
        json!([{ "id": "C-032", "from": "lore/clues/C-032.md", "to": "lore/clues/moved/C-032.md" }]),
        "{synced}"
    );
    assert!(
        listed(&synced, "changed").contains(&"lore/clues/moved/C-032.md"),
        "the file at its new path is read again: {synced}"
    );
    assert_eq!(synced["project"]["records"], RECORDS);
    let page = call(
        &window,
        "record_view",
        json!({ "id": "C-032", "path": "lore/clues/moved/C-032.md" }),
    )
    .map_err(|error| format!("the moved record has no view: {error}"))?;
    assert_eq!(
        page["header"]["record"]["path"],
        "lore/clues/moved/C-032.md"
    );
    Ok(())
}

#[test]
fn deleting_one_of_two_files_with_one_id_removes_that_file_only() -> TestResult {
    let (temp, _app, window) = opened_fixture("duplicate-id")?;
    std::fs::remove_file(temp.path().join("world/synthetic-twin.md"))?;
    let synced = sync(&window)?;
    assert_eq!(
        synced["removed"],
        json!([{ "id": "synthetic-twin", "path": "world/synthetic-twin.md" }]),
        "the file is gone although its twin keeps the id: {synced}"
    );
    assert_eq!(synced["moved"], json!([]), "{synced}");
    let gone = call(
        &window,
        "record_view",
        json!({ "id": "synthetic-twin", "path": "world/synthetic-twin.md" }),
    )
    .err()
    .ok_or("the deleted file still has a view: its path fell back to its twin")?;
    assert_eq!(gone["kind"], "not_found", "{gone}");
    let twin = call(
        &window,
        "record_view",
        json!({ "id": "synthetic-twin", "path": "world/synthetic-copy/synthetic-twin.md" }),
    )
    .map_err(|error| format!("the twin has no view: {error}"))?;
    assert_eq!(twin["header"]["record"]["title"], "Synthetic twin two");
    Ok(())
}
