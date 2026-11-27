//! `codex_graph` over stdio (CE-18.1).

use serde_json::json;
use theos_studio::codex::{self, Direction};

use crate::common::{TestResult, assert_error, client, fixture_copy, json, studio};

// AC: CE-18.1
#[tokio::test]
async fn graph_equals_the_studio_function() -> TestResult {
    let (_dir, project) = fixture_copy()?;
    let client = client(&project, &[]).await?;
    let ws = studio(&project)?;
    let defaults = client
        .structured("codex_graph", json!({ "id": "B-912" }))
        .await?;
    assert_eq!(
        defaults,
        json(&codex::graph(&ws, "B-912", 1, Direction::Both)?)?
    );
    let wide = client
        .structured(
            "codex_graph",
            json!({ "id": "mother-ilse", "depth": 3, "direction": "out" }),
        )
        .await?;
    assert_eq!(
        wide,
        json(&codex::graph(&ws, "mother-ilse", 3, Direction::Out)?)?
    );
    client.cancel().await?;
    Ok(())
}

// AC: CE-18.1
#[tokio::test]
async fn graph_bad_parameters_are_errors() -> TestResult {
    let (_dir, project) = fixture_copy()?;
    let client = client(&project, &[]).await?;
    let depth = client
        .call("codex_graph", json!({ "id": "B-912", "depth": 9 }))
        .await?;
    assert_error(&depth, "depth")?;
    let unknown = client.call("codex_graph", json!({ "id": "nope" })).await?;
    assert_error(&unknown, "nope")?;
    let direction = client
        .call("codex_graph", json!({ "id": "B-912", "direction": "up" }))
        .await;
    assert_error(&direction?, "unknown variant `up`")?;
    client.cancel().await?;
    Ok(())
}
