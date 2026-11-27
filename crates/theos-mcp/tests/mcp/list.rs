//! `codex_list` over stdio (CE-18.1).

use serde_json::json;
use theos_studio::codex::{self, ListFilter};

use crate::common::{TestResult, assert_error, client, fixture_copy, json, studio};

// AC: CE-18.1
#[tokio::test]
async fn list_equals_the_studio_function() -> TestResult {
    let (_dir, project) = fixture_copy()?;
    let client = client(&project, &[]).await?;
    let all = client.structured("codex_list", json!({})).await?;
    let got = client
        .structured(
            "codex_list",
            json!({
                "kind": "belief", "status": "draft", "folder": "lore", "offset": 1, "limit": 5
            }),
        )
        .await?;
    let ws = studio(&project)?;
    let filter = ListFilter {
        kind: None,
        status: None,
        folder: None,
        offset: 0,
        limit: 100,
    };
    assert_eq!(all, json(&codex::list(&ws, &filter)?)?);
    let filter = ListFilter {
        kind: Some("belief".into()),
        status: Some("draft".into()),
        folder: Some("lore".into()),
        offset: 1,
        limit: 5,
    };
    assert_eq!(got, json(&codex::list(&ws, &filter)?)?);
    client.cancel().await?;
    Ok(())
}

// AC: CE-18.1
#[tokio::test]
async fn list_bad_filters_are_error_results() -> TestResult {
    let (_dir, project) = fixture_copy()?;
    let client = client(&project, &[]).await?;
    let limit = client.call("codex_list", json!({ "limit": 0 })).await?;
    assert_error(&limit, "limit")?;
    let kind = client.call("codex_list", json!({ "kind": "nope" })).await?;
    assert_error(&kind, "nope")?;
    client.cancel().await?;
    Ok(())
}

// AC: CE-18.1
#[tokio::test]
async fn list_unknown_parameter_is_an_error_result() -> TestResult {
    let (_dir, project) = fixture_copy()?;
    let client = client(&project, &[]).await?;
    let result = client.call("codex_list", json!({ "kinds": "truth" })).await;
    assert_error(&result?, "unknown field `kinds`")?;
    client.cancel().await?;
    Ok(())
}
