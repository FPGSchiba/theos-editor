//! `ledger_check` over stdio (CE-18.1).

use serde_json::json;
use theos_studio::codex;

use crate::common::{TestResult, assert_error, client, fixture_copy, json, studio};

// AC: CE-18.1
#[tokio::test]
async fn ledger_equals_the_studio_function() -> TestResult {
    let (_dir, project) = fixture_copy()?;
    let client = client(&project, &[]).await?;
    let ws = studio(&project)?;
    let all = client.structured("ledger_check", json!({})).await?;
    assert_eq!(all, json(&codex::ledger_check(&ws, None)?)?);
    let one = client
        .structured("ledger_check", json!({ "id": "T-912" }))
        .await?;
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
