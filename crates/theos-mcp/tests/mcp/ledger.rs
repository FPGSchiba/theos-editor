//! `ledger_check` over stdio (CE-18.1).

use serde_json::json;
use theos_studio::codex;

use crate::common::{TestResult, assert_error, client, fixture_copy, json, studio};

// AC: CE-18.1
#[tokio::test]
async fn ledger_equals_the_studio_function() -> TestResult {
    let (_dir, project) = fixture_copy()?;
    let client = client(&project, &[]).await?;
    let all = client.structured("ledger_check", json!({})).await?;
    let one = client
        .structured("ledger_check", json!({ "id": "T-912" }))
        .await?;
    let ws = studio(&project)?;
    assert_eq!(all, json(&codex::ledger_check(&ws, None)?)?);
    assert_eq!(one, json(&codex::ledger_check(&ws, Some("T-912"))?)?);
    client.cancel().await?;
    Ok(())
}

// AC: CE-18.1
#[tokio::test]
async fn ledger_unknown_truth_is_an_error_result() -> TestResult {
    let (_dir, project) = fixture_copy()?;
    let client = client(&project, &[]).await?;
    let result = client.call("ledger_check", json!({ "id": "nope" })).await?;
    assert_error(&result, "nope")?;
    client.cancel().await?;
    Ok(())
}

// AC: CE-18.3
#[tokio::test]
async fn ledger_clues_carry_marks() -> TestResult {
    let (_dir, project) = fixture_copy()?;
    let client = client(&project, &[]).await?;
    let ledger = client.structured("ledger_check", json!({})).await?;
    let read = client
        .structured("codex_read", json!({ "id": "T-912" }))
        .await?;
    for (tool, entries) in [
        ("ledger_check", &ledger["truths"][1]["entries"]),
        ("codex_read T-912", &read["records"][0]["ledger"]["entries"]),
    ] {
        let entry = entries
            .as_array()
            .and_then(|all| all.iter().find(|e| e["clue"] == "C-031"))
            .ok_or_else(|| format!("{tool}: no entry for C-031"))?;
        assert_eq!(entry["never"], false, "{tool}: {entry}");
        assert_eq!(entry["locked"], json!([]), "{tool}: {entry}");
    }
    client.cancel().await?;
    Ok(())
}
