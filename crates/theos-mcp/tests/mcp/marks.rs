//! The marks `never` and `locked` in every result (CE-18.3).

use serde_json::{Value, json};

use crate::common::{TestResult, client, fixture_copy};

/// Every object in `value`, at any depth, whose `id` is `id` and that names a record (has `path`).
fn records_named<'a>(value: &'a Value, id: &str, found: &mut Vec<&'a Value>) {
    match value {
        Value::Object(map) => {
            if map.get("id").and_then(Value::as_str) == Some(id) && map.contains_key("path") {
                found.push(value);
            }
            map.values()
                .for_each(|inner| records_named(inner, id, found));
        }
        Value::Array(items) => items
            .iter()
            .for_each(|inner| records_named(inner, id, found)),
        _ => {}
    }
}

/// Asserts that `result` names T-911 at least `at_least` times, each time as a `never` truth.
fn assert_t911_marked(tool: &str, result: &Value, at_least: usize) {
    let mut found = Vec::new();
    records_named(result, "T-911", &mut found);
    assert!(
        found.len() >= at_least,
        "{tool}: {} mentions of T-911, wanted {at_least}: {result}",
        found.len()
    );
    for record in found {
        assert_eq!(record["never"], true, "{tool}: {record}");
        assert_eq!(record["locked"], json!(["never"]), "{tool}: {record}");
    }
}

// AC: CE-18.3
#[tokio::test]
async fn never_and_locked_in_every_result() -> TestResult {
    let (_dir, project) = fixture_copy()?;
    let client = client(&project, &[]).await?;

    let search = client
        .structured("codex_search", json!({ "query": "Synthetic statement A" }))
        .await?;
    assert_t911_marked("codex_search", &search, 1);

    // As itself, with its text in full.
    let read = client
        .structured("codex_read", json!({ "id": "T-911" }))
        .await?;
    assert_t911_marked("codex_read T-911", &read, 1);
    let text = read["records"][0]["text"].as_str().ok_or("no text")?;
    assert!(text.contains("Synthetic statement A"), "{text}");

    // As a link target.
    let belief = client
        .structured("codex_read", json!({ "id": "B-912" }))
        .await?;
    assert_t911_marked("codex_read B-912", &belief, 1);
    // As the `distorts` of a belief in a holder's lens.
    let holder = client
        .structured("codex_read", json!({ "id": "mother-ilse" }))
        .await?;
    assert_t911_marked("codex_read mother-ilse", &holder, 1);
    let beliefs = holder["records"][0]["lens"]["beliefs"]
        .as_array()
        .ok_or("no beliefs")?;
    assert!(
        beliefs.iter().any(|b| b["distorts"]["id"] == "T-911"),
        "{holder}"
    );
    let link_targets: Vec<&Value> = belief["records"][0]["links"]
        .as_array()
        .ok_or("no links")?
        .iter()
        .map(|link| &link["target"])
        .filter(|target| target["id"] == "T-911")
        .collect();
    assert!(!link_targets.is_empty(), "no link to T-911: {belief}");
    assert!(link_targets.iter().all(|t| t["never"] == true));

    let list = client
        .structured("codex_list", json!({ "kind": "truth" }))
        .await?;
    assert_t911_marked("codex_list", &list, 1);

    let graph = client
        .structured("codex_graph", json!({ "id": "B-912" }))
        .await?;
    assert_t911_marked("codex_graph", &graph, 1);

    let ledger = client.structured("ledger_check", json!({})).await?;
    assert_t911_marked("ledger_check", &ledger, 1);
    client.cancel().await?;
    Ok(())
}

// AC: CE-18.3
#[tokio::test]
async fn locked_records_are_readable_with_their_reasons() -> TestResult {
    let (_dir, project) = fixture_copy()?;
    let client = client(&project, &[]).await?;
    for (id, path, reason) in [
        ("fates-cult", "people/hearth-folk/fates-cult.md", "canon"),
        ("dup-911", "things/dup-911.md", "canon"),
        ("E91", "canon/formalism/E91.md", "formalism"),
    ] {
        let read = client.structured("codex_read", json!({ "id": id })).await?;
        let record = read["records"]
            .as_array()
            .and_then(|all| all.iter().find(|r| r["path"] == path))
            .ok_or_else(|| format!("no {path} in {read}"))?;
        let locked = record["locked"].as_array().ok_or("no locked")?;
        assert!(locked.iter().any(|r| r == reason), "{id}: {record}");
        let text = record["text"].as_str().ok_or("no text")?;
        assert!(!text.is_empty(), "{id}: empty text");
    }
    client.cancel().await?;
    Ok(())
}
