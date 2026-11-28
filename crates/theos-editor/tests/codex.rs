//! The Codex commands, called end to end through Tauri's IPC on the mock runtime.

mod support;

use serde_json::{Value, json};
use support::{TestResult, app, call, copy_fixture, default_app};
use tauri::WebviewWindow;
use tauri::test::MockRuntime;
use theos_editor::ipc::views::{QueryResult, RecordPage, TreeNode, TreeTab};
use theos_studio::{Workspace, codex_view};

type Window = WebviewWindow<MockRuntime>;

/// The fixture copied to a temporary folder and opened in the app.
fn opened()
-> Result<(tempfile::TempDir, tauri::App<MockRuntime>, Window), Box<dyn std::error::Error>> {
    opened_fixture("editor-codex")
}

/// The fixture project `name` copied to a temporary folder and opened in the app.
fn opened_fixture(
    name: &str,
) -> Result<(tempfile::TempDir, tauri::App<MockRuntime>, Window), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    copy_fixture(name, temp.path())?;
    let (app, window) = app()?;
    call(&window, "project_open", json!({ "path": temp.path() }))
        .map_err(|error| format!("the project does not open: {error}"))?;
    Ok((temp, app, window))
}

/// A call that must succeed: the answer, or the error as text.
fn ok(window: &Window, cmd: &str, args: Value) -> Result<Value, String> {
    call(window, cmd, args).map_err(|error| format!("{cmd} failed: {error}"))
}

fn fetch(window: &Window, id: &str) -> Result<Value, String> {
    ok(window, "record_view", json!({ "id": id }))
}

fn record_view(window: &Window, id: &str) -> Result<Value, Value> {
    call(window, "record_view", json!({ "id": id }))
}

fn array<'a>(value: &'a Value, what: &str) -> Result<&'a Vec<Value>, String> {
    value
        .as_array()
        .ok_or_else(|| format!("{what} is no array: {value}"))
}

/// The ids of the `source` records of the rows of `list` (Links in, Used by); none when it is no
/// array.
fn source_ids(list: &Value) -> Vec<&str> {
    list.as_array()
        .map(|rows| {
            rows.iter()
                .filter_map(|row| row["source"]["id"].as_str())
                .collect()
        })
        .unwrap_or_default()
}

// AC: CE-33.1
#[test]
fn record_view_of_the_floor_matches_screen_01() -> TestResult {
    let (_temp, _app, window) = opened()?;
    let page = fetch(&window, "f04-choir-hall")?;

    let header = &page["header"];
    assert_eq!(header["record"]["id"], "f04-choir-hall");
    assert_eq!(header["record"]["kind"], "floor");
    assert_eq!(header["record"]["status"], "draft");
    assert_eq!(header["record"]["never"], false);
    assert_eq!(header["parent"]["id"], "hearth");

    let kinds: Vec<&str> = array(&page["body"], "body")?
        .iter()
        .filter_map(|node| node["type"].as_str())
        .collect();
    assert_eq!(
        kinds,
        [
            "paragraph",
            "heading",
            "list",
            "voice",
            "design",
            "paragraph",
            "raw",
            "quote"
        ]
    );
    let chip = &page["body"][0]["inlines"]
        .as_array()
        .and_then(|inlines| inlines.iter().find(|i| i["type"] == "link"))
        .ok_or("no chip in the first paragraph")?
        .clone();
    assert_eq!(chip["id"], "fates-shrine");
    assert_eq!(chip["target"]["kind"], "location");
    assert_eq!(page["body"][3]["speaker"]["id"], "mother-ilse");
    assert_eq!(page["body"][3]["tier"], "T1");

    let lore: Vec<(&str, &str, &str)> = array(&page["lore"], "lore")?
        .iter()
        .map(|row| {
            (
                row["record"]["id"].as_str().unwrap_or_default(),
                row["verb"].as_str().unwrap_or_default(),
                row["touches"]["id"].as_str().unwrap_or_default(),
            )
        })
        .collect();
    assert_eq!(
        lore,
        [("B-017", "distorts", "T-013"), ("C-031", "hints", "T-012")]
    );
    assert_eq!(page["lore"][0]["touches"]["never"], true);
    assert!(
        page["front_matter"][0]["key"].is_string(),
        "{}",
        page["front_matter"]
    );
    assert!(
        page["findings"][0]["rule"].is_string(),
        "{}",
        page["findings"]
    );
    let link = &page["links_in"][0];
    assert!(link["line"].is_u64(), "{link}");
    Ok(())
}

// AC: CE-33.2
#[test]
fn used_by_is_derived_through_ipc() -> TestResult {
    let (temp, _app, window) = opened()?;
    let page = fetch(&window, "f04-choir-hall")?;
    assert!(source_ids(&page["used_by"]).contains(&"C-031"));
    assert!(source_ids(&page["links_in"]).contains(&"C-031"));

    std::fs::remove_file(temp.path().join("lore/clues/C-031.md"))?;
    ok(&window, "project_sync", json!({ "reason": "reload" }))?;
    let page = fetch(&window, "f04-choir-hall")?;
    assert!(!source_ids(&page["used_by"]).contains(&"C-031"));
    assert!(!source_ids(&page["links_in"]).contains(&"C-031"));
    Ok(())
}

// AC: CE-33.3
#[test]
fn search_prefixes_through_ipc() -> TestResult {
    let (_temp, _app, window) = opened()?;
    let ids = |result: &Value| -> Vec<String> {
        let mut ids: Vec<String> = result["hits"]
            .as_array()
            .map(|hits| {
                hits.iter()
                    .filter_map(|hit| hit["id"].as_str().map(str::to_owned))
                    .collect()
            })
            .unwrap_or_default();
        ids.sort();
        ids
    };
    let found = ok(
        &window,
        "codex_search",
        json!({ "query": "#f04", "limit": 20 }),
    )?;
    assert_eq!(ids(&found), ["f04-choir-hall", "f04-choir-hall-art"]);
    assert_eq!(found["truncated"], false);
    let floor = ok(
        &window,
        "codex_search",
        json!({ "query": "#F04 :floor", "limit": 20 }),
    )?;
    assert_eq!(ids(&floor), ["f04-choir-hall"]);
    let clues = ok(
        &window,
        "codex_search",
        json!({ "query": ":clue", "limit": 20 }),
    )?;
    assert_eq!(ids(&clues), ["C-031", "C-032"]);
    let none = ok(
        &window,
        "codex_search",
        json!({ "query": ":nokind", "limit": 20 }),
    )?;
    assert_eq!(none["note"], "no kind `nokind`");
    Ok(())
}

// AC: CE-33.3
#[test]
fn a_plain_word_ranks_a_title_match_first_through_ipc() -> TestResult {
    let (_temp, _app, window) = opened()?;
    // "climb" is in `B-016`'s title and in the bodies of the floor and `S-004`.
    let found = ok(
        &window,
        "codex_search",
        json!({ "query": "climb", "limit": 20 }),
    )?;
    let hits = array(&found["hits"], "hits")?;
    let ids: Vec<&str> = hits.iter().filter_map(|hit| hit["id"].as_str()).collect();
    assert_eq!(ids.first(), Some(&"B-016"), "{ids:?}");
    assert!(ids.contains(&"f04-choir-hall"), "{ids:?}");
    assert_eq!(hits[0]["matches"][0]["column"], "title");
    Ok(())
}

#[test]
fn a_record_whose_file_is_gone_before_a_sync_is_an_io_error() -> TestResult {
    let (temp, _app, window) = opened()?;
    std::fs::remove_file(
        temp.path()
            .join("world/hearth/f04-choir-hall/reed-marsh.md"),
    )?;
    let error = record_view(&window, "reed-marsh")
        .err()
        .ok_or("the record opened without its file")?;
    assert_eq!(error["kind"], "io", "{error}");
    assert!(
        error["message"]
            .as_str()
            .is_some_and(|message| message.matches("reed-marsh.md").count() == 1),
        "the message names the file once: {error}"
    );
    Ok(())
}

#[test]
fn a_call_with_missing_arguments_fails_without_running() -> TestResult {
    let (_temp, _app, window) = opened()?;
    for (cmd, args) in [
        ("record_view", json!({})),
        ("codex_search", json!({ "query": "x" })),
        ("codex_tree", json!({ "tab": "nowhere" })),
    ] {
        let error = call(&window, cmd, args)
            .err()
            .ok_or_else(|| format!("{cmd} ran without its arguments"))?;
        assert!(error.is_string(), "{cmd}: {error}");
    }
    Ok(())
}

// AC: CE-33.1
#[test]
fn codex_tree_returns_the_tabs() -> TestResult {
    let (_temp, _app, window) = opened()?;
    let places = ok(&window, "codex_tree", json!({ "tab": "where" }))?;
    let ids: Vec<&str> = array(&places, "places")?
        .iter()
        .filter_map(|node| node["record"]["id"].as_str())
        .collect();
    assert!(ids.contains(&"tower"), "{ids:?}");
    let lore = ok(&window, "codex_tree", json!({ "tab": "lore" }))?;
    assert_eq!(lore[0]["type"], "group");
    assert_eq!(lore[0]["label"], "Truths");
    assert!(call(&window, "codex_tree", json!({ "tab": "nowhere" })).is_err());
    Ok(())
}

// AC: CE-33.1
#[test]
fn record_view_opens_the_file_at_the_path() -> TestResult {
    let (_temp, _app, window) = opened_fixture("duplicate-id")?;
    for (path, title) in [
        (json!("world/synthetic-twin.md"), "Synthetic twin one"),
        (
            json!("world/synthetic-copy/synthetic-twin.md"),
            "Synthetic twin two",
        ),
        (Value::Null, "Synthetic twin two"),
    ] {
        let args = json!({ "id": "synthetic-twin", "path": path });
        let page = ok(&window, "record_view", args)?;
        assert_eq!(page["header"]["record"]["title"], title, "{path}");
    }
    Ok(())
}

#[test]
fn unknown_id_is_not_found() -> TestResult {
    let (_temp, _app, window) = opened()?;
    let error = record_view(&window, "no-such-record")
        .err()
        .ok_or("an unknown id gave a page")?;
    assert_eq!(error["kind"], "not_found", "{error}");
    assert!(
        error["message"]
            .as_str()
            .is_some_and(|message| message.contains("no-such-record")),
        "{error}"
    );
    Ok(())
}

#[test]
fn codex_commands_need_a_project() -> TestResult {
    // The default app: nothing is opened, so no state file is written.
    let (_app, window) = default_app()?;
    for (cmd, args) in [
        ("codex_tree", json!({ "tab": "where" })),
        ("record_view", json!({ "id": "x" })),
        ("codex_search", json!({ "query": "x", "limit": 5 })),
    ] {
        let error = call(&window, cmd, args)
            .err()
            .ok_or("a call without a project worked")?;
        assert_eq!(error["kind"], "no_project", "{cmd}: {error}");
    }
    Ok(())
}

fn to_json(value: &impl serde::Serialize) -> Result<Value, serde_json::Error> {
    serde_json::to_value(value)
}

/// The IPC mirrors serialise to the studio's JSON, which the recorded answers rely on.
#[test]
fn mirrors_serialise_like_the_studio() -> TestResult {
    let temp = tempfile::tempdir()?;
    copy_fixture("editor-codex", temp.path())?;
    let workspace = Workspace::open(temp.path())?;

    let page = codex_view::record(&workspace, "f04-choir-hall", None)?;
    let studio = to_json(&page)?;
    assert_eq!(to_json(&RecordPage::from(page))?, studio, "record page");

    let found = codex_view::query(&workspace, ":clue", 20)?;
    let studio = to_json(&found)?;
    assert!(
        studio["hits"]
            .as_array()
            .is_some_and(|hits| !hits.is_empty()),
        "{studio}"
    );
    assert_eq!(to_json(&QueryResult::from(found))?, studio, "query `:clue`");

    for tab in [TreeTab::Where, TreeTab::Who, TreeTab::Lore, TreeTab::Things] {
        let nodes = codex_view::tree(&workspace, tab.into())?;
        let studio = to_json(&nodes)?;
        let mirrors: Vec<TreeNode> = nodes.into_iter().map(TreeNode::from).collect();
        assert_eq!(to_json(&mirrors)?, studio, "tree tab {tab:?}");
    }
    Ok(())
}

/// The kinds of every record row under `rows`.
fn row_kinds(rows: &Value) -> Vec<String> {
    let mut kinds = Vec::new();
    for row in rows.as_array().into_iter().flatten() {
        if let Some(kind) = row["record"]["kind"].as_str() {
            kinds.push(kind.to_owned());
        }
        kinds.extend(row_kinds(&row["children"]));
    }
    kinds
}

#[test]
fn codex_tabs_is_the_table_the_tree_uses() -> TestResult {
    let (_temp, _app, window) = opened()?;
    let answer = ok(&window, "codex_tabs", json!({}))?;
    let tabs = array(&answer, "codex_tabs")?;
    assert_eq!(tabs.len(), codex_view::TABS.len());
    for (info, entry) in tabs.iter().zip(codex_view::TABS) {
        assert_eq!(info["tab"], json!(entry.tab));
        let table = codex_view::table(entry.tab);
        assert_eq!(table.tab, entry.tab, "table() finds the entry of its tab");
        assert_eq!(info["kinds"], json!(table.kinds()));
        assert_eq!(info["folder"], json!(table.folder));
        assert_eq!(info["root"], json!(table.root()));
        let id_first: Vec<&str> = if table.id_first {
            table.kinds()
        } else {
            Vec::new()
        };
        assert_eq!(info["id_first"], json!(id_first));
        let rows = ok(&window, "codex_tree", json!({ "tab": info["tab"] }))?;
        for kind in row_kinds(&rows) {
            assert!(
                table.kinds().contains(&kind.as_str()),
                "{kind} shows in {} but is not one of its kinds",
                info["tab"]
            );
        }
    }
    let lore = tabs.iter().find(|info| info["tab"] == "lore");
    assert_eq!(
        lore.map(|info| info["id_first"].clone()),
        Some(json!(["topic", "truth", "belief", "clue", "equation"]))
    );
    Ok(())
}
