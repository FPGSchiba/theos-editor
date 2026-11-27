//! `codex_search` over stdio (CE-18.1).

use serde_json::json;
use theos_studio::codex;

use crate::common::{TestResult, assert_error, client, fixture_copy, json, studio};

// AC: CE-18.1
#[tokio::test]
async fn search_equals_the_studio_function() -> TestResult {
    let (_dir, project) = fixture_copy()?;
    let client = client(&project, &[]).await?;
    let got = client
        .structured("codex_search", json!({ "query": "Mother Ilse" }))
        .await?;
    let want = json(&codex::search(&studio(&project)?, "Mother Ilse", 20)?)?;
    assert_eq!(got, want);
    assert!(
        got["hits"].as_array().map_or(0, Vec::len) > 0,
        "no hits: {got}"
    );
    client.cancel().await?;
    Ok(())
}

// AC: CE-18.1
#[tokio::test]
async fn search_limit_applies() -> TestResult {
    let (_dir, project) = fixture_copy()?;
    let client = client(&project, &[]).await?;
    let got = client
        .structured("codex_search", json!({ "query": "synthetic", "limit": 1 }))
        .await?;
    let want = json(&codex::search(&studio(&project)?, "synthetic", 1)?)?;
    assert_eq!(got, want);
    assert_eq!(got["hits"].as_array().map(Vec::len), Some(1));
    assert_eq!(got["truncated"], true);
    client.cancel().await?;
    Ok(())
}

// AC: CE-18.1
#[tokio::test]
async fn search_limit_zero_is_an_error_result() -> TestResult {
    let (_dir, project) = fixture_copy()?;
    let client = client(&project, &[]).await?;
    let result = client
        .call("codex_search", json!({ "query": "x", "limit": 0 }))
        .await?;
    assert_error(&result, "limit")?;
    client.cancel().await?;
    Ok(())
}

// AC: CE-18.1
#[tokio::test]
async fn search_unknown_parameter_is_an_error_result() -> TestResult {
    let (_dir, project) = fixture_copy()?;
    let client = client(&project, &[]).await?;
    let result = client
        .call("codex_search", json!({ "query": "x", "limt": 3 }))
        .await;
    assert_error(&result?, "unknown field `limt`")?;
    client.cancel().await?;
    Ok(())
}
