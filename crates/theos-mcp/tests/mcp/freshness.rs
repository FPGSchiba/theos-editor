//! A project edited on disk between two calls (CE-18.1, spec §5.3).

use std::path::Path;
use std::time::Duration;

use serde_json::{Value, json};

use crate::common::{Client, TestResult, assert_error, client, fixture_copy};

/// Replaces `from` by `to` in the file `relative` of `project`, once, and lets the clock move on
/// so the change is also a new modification time. The replacement has another length.
async fn edit(project: &Path, relative: &str, from: &str, to: &str) -> TestResult {
    assert_ne!(from.len(), to.len(), "an edit must change the size");
    let path = project.join(relative);
    let text = std::fs::read_to_string(&path)?;
    assert!(text.contains(from), "{relative} has no `{from}`");
    std::fs::write(&path, text.replacen(from, to, 1))?;
    tokio::time::sleep(Duration::from_millis(50)).await;
    Ok(())
}

/// Renames the clue file `from` to `to` and sets the id inside it.
fn edit_id(project: &Path, from: &str, to: &str) -> TestResult {
    let old = project.join(format!("lore/clues/{from}.md"));
    let text = std::fs::read_to_string(&old)?;
    std::fs::write(
        project.join(format!("lore/clues/{to}.md")),
        text.replacen(&format!("id: {from}"), &format!("id: {to}"), 1),
    )?;
    std::fs::remove_file(old)?;
    Ok(())
}

async fn read(client: &Client, id: &str) -> Result<Value, crate::common::Error> {
    client.structured("codex_read", json!({ "id": id })).await
}

// AC: CE-18.1
#[tokio::test]
async fn edits_on_disk_show_in_next_call() -> TestResult {
    let (_dir, project) = fixture_copy()?;
    let client = client(&project, &[]).await?;

    // 1. A title changes.
    let before = read(&client, "B-911").await?;
    assert_eq!(before["records"][0]["title"], "Synthetic belief A");
    edit(
        &project,
        "lore/beliefs/B-911.md",
        "title: Synthetic belief A",
        "title: Synthetic belief A, renamed",
    )
    .await?;
    let after = read(&client, "B-911").await?;
    assert_eq!(after["records"][0]["title"], "Synthetic belief A, renamed");

    // 2. A truth gets target `never`.
    let before = read(&client, "T-912").await?;
    assert_eq!(before["records"][0]["never"], false);
    edit(
        &project,
        "lore/truths/T-912.md",
        "target: hinted",
        "target: never",
    )
    .await?;
    let after = read(&client, "T-912").await?;
    assert_eq!(after["records"][0]["never"], true);
    assert_eq!(after["records"][0]["locked"], json!(["never"]));

    // 3. The project's settings change a rule severity.
    let severity = |info: &Value| {
        info["rules"]
            .as_array()
            .and_then(|rules| rules.iter().find(|r| r["id"] == "lore/three-clue"))
            .map(|rule| rule["severity"].clone())
    };
    let before = client.structured("project_info", json!({})).await?;
    assert_eq!(severity(&before), Some(json!("warning")));
    std::fs::write(
        project.join(".theos/settings.toml"),
        "[validate.severity]\n\"lore/three-clue\" = \"error\"\n",
    )?;
    tokio::time::sleep(Duration::from_millis(50)).await;
    let after = client.structured("project_info", json!({})).await?;
    assert_eq!(severity(&after), Some(json!("error")));

    // 4. A clue is renamed. The id is the file name, so the id of the file and its content
    // change together; C-032 is taken, so C-034 it is.
    read(&client, "C-031").await?;
    edit_id(&project, "C-031", "C-034")?;
    let after = read(&client, "C-034").await?;
    assert_eq!(after["records"][0]["path"], "lore/clues/C-034.md");
    let gone = client.call("codex_read", json!({ "id": "C-031" })).await?;
    assert_error(&gone, "no record `C-031`")?;

    // 5. A record is deleted.
    read(&client, "T-912").await?;
    std::fs::remove_file(project.join("lore/truths/T-912.md"))?;
    let gone = client.call("codex_read", json!({ "id": "T-912" })).await?;
    assert_error(&gone, "no record `T-912`")?;
    client.cancel().await?;
    Ok(())
}
