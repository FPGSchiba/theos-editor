//! The schema walk of spec §5.6 over the result of every tool (CE-18.3).

use serde_json::{Value, json};

use crate::common::{TestResult, client, fixture_copy};

/// Collects into `bad` the place of every object that names a record (a string `id` and a
/// `path` key) but lacks `never` (a bool) or `locked` (an array).
fn walk(value: &Value, at: &str, bad: &mut Vec<String>) {
    match value {
        Value::Object(map) => {
            // A finding also has an `id` and a `path`, but it is not one of the record references
            // spec 5.6 lists; it is told apart by its `rule`.
            let names_record = map.get("id").is_some_and(Value::is_string)
                && map.contains_key("path")
                && !map.contains_key("rule");
            let marked = map.get("never").is_some_and(Value::is_boolean)
                && map.get("locked").is_some_and(Value::is_array);
            if names_record && !marked {
                bad.push(format!("{at}: {value}"));
            }
            for (key, inner) in map {
                walk(inner, &format!("{at}.{key}"), bad);
            }
        }
        Value::Array(items) => {
            for (index, inner) in items.iter().enumerate() {
                walk(inner, &format!("{at}[{index}]"), bad);
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
        ("ledger_check", json!({})),
    ];
    let mut bad = Vec::new();
    for (tool, arguments) in calls {
        let result = client.structured(tool, arguments.clone()).await?;
        walk(&result, &format!("{tool} {arguments}"), &mut bad);
    }
    assert!(bad.is_empty(), "records without marks:\n{}", bad.join("\n"));
    client.cancel().await?;
    Ok(())
}
