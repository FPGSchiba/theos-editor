//! `codex_read` over stdio (CE-18.1, CE-18.2).

use serde_json::json;
use theos_studio::codex::{self, ReadTarget};

use crate::common::{TestResult, assert_error, client, fixture_copy, json, studio};

// AC: CE-18.1
#[tokio::test]
async fn read_by_id_and_by_path_equal_the_studio_function() -> TestResult {
    let (_dir, project) = fixture_copy()?;
    let client = client(&project, &[]).await?;
    let by_id = client
        .structured("codex_read", json!({ "id": "mother-ilse" }))
        .await?;
    let path = "lore/beliefs/B-911.md";
    let by_path = client
        .structured("codex_read", json!({ "path": path }))
        .await?;
    let ws = studio(&project)?;
    let want = json(&codex::read(&ws, &ReadTarget::Id("mother-ilse".into()))?)?;
    assert_eq!(by_id, want);
    let want = json(&codex::read(&ws, &ReadTarget::Path(path.into()))?)?;
    assert_eq!(by_path, want);
    client.cancel().await?;
    Ok(())
}

// AC: CE-18.1
#[tokio::test]
async fn read_needs_exactly_one_of_id_and_path() -> TestResult {
    let (_dir, project) = fixture_copy()?;
    let client = client(&project, &[]).await?;
    let both = client
        .call(
            "codex_read",
            json!({ "id": "B-911", "path": "lore/beliefs/B-911.md" }),
        )
        .await?;
    assert_error(&both, "give exactly one of `id` and `path`")?;
    let neither = client.call("codex_read", json!({})).await?;
    assert_error(&neither, "give exactly one of `id` and `path`")?;
    client.cancel().await?;
    Ok(())
}

// AC: CE-18.1
#[tokio::test]
async fn read_unknown_id_is_an_error_result() -> TestResult {
    let (_dir, project) = fixture_copy()?;
    let client = client(&project, &[]).await?;
    let result = client.call("codex_read", json!({ "id": "nope" })).await?;
    assert_error(&result, "no record `nope`")?;
    client.cancel().await?;
    Ok(())
}

// AC: CE-18.2
#[tokio::test]
async fn mother_ilse_from_records_alone() -> TestResult {
    let (_dir, project) = fixture_copy()?;
    let client = client(&project, &[]).await?;
    let search = client
        .structured("codex_search", json!({ "query": "Mother Ilse" }))
        .await?;
    let ids: Vec<&str> = search["hits"]
        .as_array()
        .ok_or("no hits")?
        .iter()
        .filter_map(|hit| hit["id"].as_str())
        .collect();
    assert!(ids.contains(&"mother-ilse"), "{ids:?}");

    let read = client
        .structured("codex_read", json!({ "id": "mother-ilse" }))
        .await?;
    let lens = &read["records"][0]["lens"];
    let chain: Vec<&str> = lens["chain"]
        .as_array()
        .ok_or("no chain")?
        .iter()
        .filter_map(|link| link["id"].as_str())
        .collect();
    assert_eq!(chain, ["mother-ilse", "fates-cult", "hearth-folk"]);
    let belief = |id: &str| {
        lens["beliefs"]
            .as_array()
            .and_then(|all| all.iter().find(|b| b["belief"]["id"] == id))
            .ok_or_else(|| format!("no belief {id}"))
    };
    let b911 = belief("B-911")?;
    assert_eq!(b911["through"]["id"], "fates-cult");
    assert_eq!(b911["belief"]["title"], "Synthetic belief A");
    assert_eq!(b911["distorts"]["id"], "T-912");
    let b912 = belief("B-912")?;
    assert_eq!(b912["through"]["id"], "mother-ilse");
    assert_eq!(b912["belief"]["title"], "Synthetic belief B");
    assert_eq!(b912["distorts"]["id"], "T-911");

    let belief = client
        .structured("codex_read", json!({ "id": "B-911" }))
        .await?;
    let text = belief["records"][0]["text"].as_str().ok_or("no text")?;
    assert!(text.contains("Synthetic belief A"), "{text}");
    client.cancel().await?;
    Ok(())
}
