//! The app's project commands, called end to end through Tauri's IPC on the mock runtime.

mod support;

use std::path::Path;

use serde_json::{Value, json};
use support::{TestResult, app, app_with, call, copy_fixture, default_app};
use theos_editor::AppState;

/// The rail of the fixture, whose `enabled` is tower-of-theos's.
const RAIL: [&str; 9] = [
    "codex", "graph", "scripts", "concepts", "maps", "review", "changes", "code", "settings",
];

/// The records of the fixture.
const RECORDS: u64 = 22;

/// Opens the project in `root`: its view, or the error as text.
fn open(
    window: &tauri::WebviewWindow<tauri::test::MockRuntime>,
    root: &Path,
) -> Result<Value, String> {
    call(window, "project_open", json!({ "path": root }))
        .map_err(|error| format!("the project does not open: {error}"))
}

fn rail_ids(project: &Value) -> Vec<&str> {
    project["rail"]
        .as_array()
        .map(|rail| {
            rail.iter()
                .filter_map(|entry| entry["id"].as_str())
                .collect()
        })
        .unwrap_or_default()
}

// AC: CE-32.1
#[test]
fn open_returns_the_rail_of_enabled_parts() -> TestResult {
    let temp = tempfile::tempdir()?;
    copy_fixture("editor-codex", temp.path())?;
    let (_app, window) = app()?;
    let project = open(&window, temp.path())?;
    assert_eq!(project["name"], "synthetic-codex");
    assert_eq!(project["kind"], "content");
    assert_eq!(rail_ids(&project), RAIL);
    assert_eq!(
        project["rail"][0],
        json!({ "id": "codex", "label": "Codex", "glyph": "◈", "order": 10, "phase": null })
    );
    assert_eq!(project["rail"][5]["phase"], "P13");
    assert_eq!(project["records"], RECORDS);
    // Errors: the dangling and the unclosed link and the malformed voice title of the floor, the
    // dangling parent of `synthetic-stray` and the parent cycle of the two loop locations.
    // Warnings: two clues where the three-clue rule asks for three, and the misplaced loop
    // location.
    assert_eq!(project["errors"], 6, "{project}");
    assert_eq!(project["warnings"], 2, "{project}");
    assert_eq!(project["in_memory"], false);
    Ok(())
}

// AC: CE-32.1
#[test]
fn open_a_folder_without_theos_toml_is_not_a_project() -> TestResult {
    let temp = tempfile::tempdir()?;
    let project = temp.path().join("project");
    copy_fixture("editor-codex", &project)?;
    let empty = temp.path().join("empty");
    std::fs::create_dir(&empty)?;
    let (_app, window) = app()?;
    open(&window, &project)?;
    let error = call(&window, "project_open", json!({ "path": empty }))
        .err()
        .ok_or("a folder without theos.toml opened")?;
    assert_eq!(error["kind"], "not_a_project", "{error}");
    let message = error["message"].as_str().unwrap_or_default();
    assert!(message.contains("has no theos.toml"), "{error}");
    assert!(message.contains(&empty.display().to_string()), "{error}");
    let synced =
        call(&window, "project_sync", json!({ "reason": "reload" })).map_err(|e| e.to_string())?;
    assert_eq!(
        synced["project"]["name"], "synthetic-codex",
        "the open project stays open"
    );
    Ok(())
}

#[test]
fn commands_before_open_fail_with_no_project() -> TestResult {
    let (_app, window) = app()?;
    assert_eq!(
        call(&window, "project_sync", json!({ "reason": "reload" })),
        Err(json!({ "kind": "no_project", "message": "no project is open" }))
    );
    assert_eq!(call(&window, "project_last", json!({})), Ok(Value::Null));
    Ok(())
}

// AC: CE-32.3
#[test]
fn engine_feature_state() -> TestResult {
    let (_app, window) = default_app()?;
    let info = call(&window, "app_info", json!({})).map_err(|e| e.to_string())?;
    assert_eq!(info["engine"], cfg!(feature = "engine"));
    assert_eq!(info["version"], env!("CARGO_PKG_VERSION"));
    assert_eq!(info["os"], std::env::consts::OS);
    #[cfg(feature = "engine")]
    assert_eq!(theos_editor::presets::engine_presets(), Vec::<&str>::new());
    assert_eq!(
        theos_editor::presets::content_presets().first(),
        Some(&"codex")
    );
    let metadata = cargo_metadata::MetadataCommand::new()
        .manifest_path(concat!(env!("CARGO_MANIFEST_DIR"), "/Cargo.toml"))
        .no_deps()
        .exec()?;
    let package = metadata
        .packages
        .iter()
        .find(|package| package.name.as_str() == "theos-editor")
        .ok_or("no theos-editor package")?;
    assert_eq!(
        package.features.get("engine"),
        Some(&Vec::new()),
        "the feature `engine` stays empty"
    );
    Ok(())
}

#[test]
fn path_with_spaces_and_non_ascii_opens() -> TestResult {
    let temp = tempfile::tempdir()?;
    let root = temp.path().join("Projekt Ä");
    copy_fixture("editor-codex", &root)?;
    let (_app, window) = app()?;
    let project = open(&window, &root)?;
    assert_eq!(project["root"], root.display().to_string());
    assert_eq!(project["records"], RECORDS);
    let synced =
        call(&window, "project_sync", json!({ "reason": "reload" })).map_err(|e| e.to_string())?;
    assert_eq!(synced["changed"], json!([]));
    Ok(())
}

#[test]
fn sync_reports_changed_and_removed_records() -> TestResult {
    let temp = tempfile::tempdir()?;
    copy_fixture("editor-codex", temp.path())?;
    let (_app, window) = app()?;
    open(&window, temp.path())?;
    let truth = temp.path().join("lore/truths/T-013.md");
    let text = std::fs::read_to_string(&truth)?;
    std::fs::write(
        &truth,
        text.replace("Synthetic text.", "Synthetic text, changed."),
    )?;
    std::fs::remove_file(temp.path().join("lore/clues/C-032.md"))?;
    let synced =
        call(&window, "project_sync", json!({ "reason": "reload" })).map_err(|e| e.to_string())?;
    let changed = synced["changed"].as_array().ok_or("no changed list")?;
    assert!(
        changed.contains(&json!({ "id": "T-013", "path": "lore/truths/T-013.md" })),
        "{synced}"
    );
    assert_eq!(
        synced["removed"],
        json!([{ "id": "C-032", "path": "lore/clues/C-032.md" }])
    );
    assert_eq!(synced["project"]["records"], RECORDS - 1);
    Ok(())
}

#[test]
fn state_file_round_trips() -> TestResult {
    let temp = tempfile::tempdir()?;
    let root = temp.path().join("project");
    copy_fixture("editor-codex", &root)?;
    let data = temp.path().join("data");
    let (_app, window) = app_with(AppState::new(Some(data.clone())))?;
    assert_eq!(call(&window, "project_last", json!({})), Ok(Value::Null));
    open(&window, &root)?;
    let last = json!(root.display().to_string());
    assert_eq!(call(&window, "project_last", json!({})), Ok(last.clone()));
    assert!(data.join(theos_editor::state::FILE_NAME).is_file());
    let (_again, window) = app_with(AppState::new(Some(data)))?;
    assert_eq!(call(&window, "project_last", json!({})), Ok(last));
    Ok(())
}

// AC: CE-32.1
#[test]
fn the_command_line_argument_is_the_project_to_open() -> TestResult {
    let temp = tempfile::tempdir()?;
    let root = temp.path().join("Projekt Ä");
    copy_fixture("editor-codex", &root)?;
    let (_app, window) = app_with(AppState::new(None).with_argument(Some(root.clone())))?;
    let argument = call(&window, "project_argument", json!({})).map_err(|e| e.to_string())?;
    assert_eq!(argument, json!(root.display().to_string()));
    let path = argument.as_str().ok_or("the argument is not a path")?;
    let project = open(&window, Path::new(path))?;
    assert_eq!(project["records"], RECORDS);
    Ok(())
}

#[test]
fn without_an_argument_there_is_none() -> TestResult {
    let (_app, window) = app()?;
    assert_eq!(
        call(&window, "project_argument", json!({})),
        Ok(Value::Null)
    );
    Ok(())
}

#[test]
fn the_argument_is_the_first_after_the_program() {
    let args = ["theos-editor", "synthetic project", "more"].map(std::ffi::OsString::from);
    assert_eq!(
        theos_editor::argument(args),
        Some(std::path::PathBuf::from("synthetic project"))
    );
    assert_eq!(
        theos_editor::argument([std::ffi::OsString::from("theos-editor")]),
        None
    );
}

// AC: CE-32.1
#[test]
fn a_missing_or_file_argument_does_not_open() -> TestResult {
    let temp = tempfile::tempdir()?;
    let file = temp.path().join("theos.toml");
    std::fs::write(&file, "")?;
    for argument in [temp.path().join("missing"), file] {
        let (_app, window) = app_with(AppState::new(None).with_argument(Some(argument.clone())))?;
        let named = call(&window, "project_argument", json!({})).map_err(|e| e.to_string())?;
        assert_eq!(named, json!(argument.display().to_string()));
        let error = call(&window, "project_open", json!({ "path": argument }))
            .err()
            .ok_or("a path that is not a folder opened")?;
        assert_eq!(error["kind"], "not_a_project", "{error}");
        assert!(
            error["message"]
                .as_str()
                .is_some_and(|m| m.contains(&argument.display().to_string())),
            "{error}"
        );
        assert_eq!(
            call(&window, "project_sync", json!({ "reason": "reload" }))
                .err()
                .map(|e| e["kind"].clone()),
            Some(json!("no_project"))
        );
    }
    Ok(())
}

#[test]
fn an_empty_path_is_an_io_error() -> TestResult {
    let (_app, window) = app()?;
    let error = call(&window, "project_open", json!({ "path": "" }))
        .err()
        .ok_or("an empty path opened")?;
    assert_eq!(error["kind"], "io", "{error}");
    Ok(())
}

#[test]
fn a_state_file_that_cannot_be_written_does_not_stop_the_open() -> TestResult {
    let temp = tempfile::tempdir()?;
    let root = temp.path().join("project");
    copy_fixture("editor-codex", &root)?;
    // A file where the data folder should be: the state file cannot be written.
    let blocked = temp.path().join("synthetic-file");
    std::fs::write(&blocked, "")?;
    let (_app, window) = app_with(AppState::new(Some(blocked.join("theos"))))?;
    let project = open(&window, &root)?;
    assert_eq!(project["records"], RECORDS);
    assert_eq!(call(&window, "project_last", json!({})), Ok(Value::Null));
    Ok(())
}

#[test]
fn a_project_with_an_unknown_part_opens_without_it() -> TestResult {
    let temp = tempfile::tempdir()?;
    std::fs::write(
        temp.path().join("theos.toml"),
        "[project]\nname = \"synthetic-parts\"\nkind = \"content\"\nschema_version = 1\n\n\
         [parts]\nenabled = [\"codex\", \"synthetic-nonsense\"]\n",
    )?;
    let (_app, window) = app()?;
    let project = open(&window, temp.path())?;
    assert_eq!(rail_ids(&project), ["codex", "code", "settings"]);
    Ok(())
}

#[test]
fn project_calls_with_bad_arguments_fail_without_running() -> TestResult {
    let (_app, window) = app()?;
    for (cmd, args) in [
        ("project_open", json!({})),
        ("project_sync", json!({ "reason": "synthetic" })),
    ] {
        let error = call(&window, cmd, args)
            .err()
            .ok_or_else(|| format!("{cmd} ran with bad arguments"))?;
        assert!(error.is_string(), "{cmd}: {error}");
    }
    Ok(())
}

#[test]
fn a_project_without_a_name_takes_its_folder_name() -> TestResult {
    let temp = tempfile::tempdir()?;
    let root = temp.path().join("Synthetic Projekt Ä");
    std::fs::create_dir(&root)?;
    std::fs::write(
        root.join("theos.toml"),
        "[project]\nkind = \"content\"\nschema_version = 1\n",
    )?;
    let (_app, window) = app()?;
    let project = open(&window, &root)?;
    assert_eq!(project["name"], "Synthetic Projekt Ä");
    Ok(())
}
