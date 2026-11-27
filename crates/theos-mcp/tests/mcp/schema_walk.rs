//! The schema walk of spec §5.6 over the result of every tool (CE-18.3).

use std::collections::HashSet;

use serde_json::{Value, json};

use crate::common::{TestResult, client, fixture_copy};

/// Collects into `bad` the place of every object that names a record (a string `id` and a
/// `path` key) but lacks `never` (a bool) or `locked` (an array), and of every record named by a
/// bare id: a string under `place` or `script`, or an array of strings that are all ids of
/// records of the project (`ids`).
fn walk(value: &Value, ids: &HashSet<String>, at: &str, bad: &mut Vec<String>) {
    match value {
        Value::Object(map) => {
            // A finding also has an `id` and a `path`, but it is not one of the record references
            // spec 5.6 lists; its shape (`rule`, `severity`, `message`) tells it apart. A ledger
            // clue names a record with `clue` and `clue_path`.
            let finding = ["rule", "severity", "message"]
                .iter()
                .all(|key| map.contains_key(*key));
            let names_record = !finding
                && ((map.get("id").is_some_and(Value::is_string) && map.contains_key("path"))
                    || (map.get("clue").is_some_and(Value::is_string)
                        && map.contains_key("clue_path")));
            let marked = map.get("never").is_some_and(Value::is_boolean)
                && map.get("locked").is_some_and(Value::is_array);
            if names_record && !marked {
                bad.push(format!("{at}: {value}"));
            }
            for (key, inner) in map {
                if matches!(key.as_str(), "place" | "script") && inner.is_string() {
                    bad.push(format!("{at}.{key}: bare id {inner}"));
                }
                walk(inner, ids, &format!("{at}.{key}"), bad);
            }
        }
        Value::Array(items) => {
            let bare = !items.is_empty()
                && items
                    .iter()
                    .all(|item| item.as_str().is_some_and(|id| ids.contains(id)));
            if bare {
                bad.push(format!("{at}: bare ids {value}"));
            }
            for (index, inner) in items.iter().enumerate() {
                walk(inner, ids, &format!("{at}[{index}]"), bad);
            }
        }
        _ => {}
    }
}

// AC: CE-18.3
#[tokio::test]
async fn every_record_object_carries_marks() -> TestResult {
    let (_dir, project) = fixture_copy()?;
    let client = client(&project, &[]).await?;
    let all = client
        .structured("codex_list", json!({ "limit": 500 }))
        .await?;
    let ids: HashSet<String> = all["records"]
        .as_array()
        .ok_or("no records")?
        .iter()
        .filter_map(|record| record["id"].as_str().map(str::to_owned))
        .collect();
    assert!(ids.contains("T-911") && ids.contains("f04-synthetic"));
    let calls = [
        ("project_info", json!({})),
        ("codex_search", json!({ "query": "synthetic" })),
        ("codex_read", json!({ "id": "mother-ilse" })),
        ("codex_read", json!({ "id": "T-911" })),
        ("codex_read", json!({ "id": "B-912" })),
        // S-004 links the missing record `ghost-911`.
        ("codex_read", json!({ "id": "S-004" })),
        ("codex_list", json!({})),
        ("codex_graph", json!({ "id": "mother-ilse", "depth": 3 })),
        ("codex_graph", json!({ "id": "S-004" })),
        ("codex_read", json!({ "id": "fates-cult" })),
        ("ledger_check", json!({})),
    ];
    let mut bad = Vec::new();
    for (tool, arguments) in calls {
        let result = client.structured(tool, arguments.clone()).await?;
        walk(&result, &ids, &format!("{tool} {arguments}"), &mut bad);
    }
    assert!(bad.is_empty(), "records without marks:\n{}", bad.join("\n"));
    client.cancel().await?;
    Ok(())
}
