//! `project_info` and the server's start-up behaviour (CE-18.1).

use std::io::Write;
use std::process::{Command, Stdio};

use rmcp::model::CallToolRequestParams;

use crate::common::{TestResult, client, copy_dir, fixture, fixture_copy, structured, text};

/// The records of the `codex` fixture.
const FIXTURE_RECORDS: u64 = 18;

// AC: CE-18.1
#[tokio::test]
async fn lists_exactly_the_tools() -> TestResult {
    let (_dir, project) = fixture_copy()?;
    let client = client(&project, &[]).await?;
    let tools = client.list_all_tools().await?;
    let names: Vec<&str> = tools.iter().map(|tool| &*tool.name).collect();
    assert_eq!(names, ["project_info"]);
    let tool = &tools[0];
    assert!(tool.output_schema.is_some(), "no output schema");
    let read_only = tool.annotations.as_ref().and_then(|a| a.read_only_hint);
    assert_eq!(read_only, Some(true));
    client.cancel().await?;
    Ok(())
}

// AC: CE-18.1
#[tokio::test]
async fn project_info_over_stdio() -> TestResult {
    let (_dir, project) = fixture_copy()?;
    let client = client(&project, &[]).await?;
    let result = client
        .call_tool(CallToolRequestParams::new("project_info"))
        .await?;
    assert_ne!(result.is_error, Some(true), "{result:?}");
    let value = structured(&result)?;
    assert_eq!(value["cache"], "file");
    assert_eq!(value["records"], FIXTURE_RECORDS);
    client.cancel().await?;
    Ok(())
}

// AC: CE-18.1
#[tokio::test]
async fn start_outside_project_reports_then_recovers() -> TestResult {
    let dir = tempfile::tempdir()?;
    let client = client(dir.path(), &[]).await?;
    let first = client
        .call_tool(CallToolRequestParams::new("project_info"))
        .await?;
    assert_eq!(first.is_error, Some(true));
    assert!(text(&first)?.contains("no theos.toml"), "{first:?}");
    copy_dir(&fixture(), dir.path())?;
    let second = client
        .call_tool(CallToolRequestParams::new("project_info"))
        .await?;
    assert_ne!(second.is_error, Some(true), "{second:?}");
    assert_eq!(structured(&second)?["records"], FIXTURE_RECORDS);
    client.cancel().await?;
    Ok(())
}

// AC: CE-18.1
#[tokio::test]
async fn given_project_must_hold_theos_toml() -> TestResult {
    let (_dir, project) = fixture_copy()?;
    let empty = tempfile::tempdir()?;
    let folder = empty.path().to_string_lossy().into_owned();
    let client = client(&project, &["--project", &folder]).await?;
    let result = client
        .call_tool(CallToolRequestParams::new("project_info"))
        .await?;
    assert_eq!(result.is_error, Some(true));
    let message = text(&result)?;
    assert!(message.contains("theos.toml"), "{message}");
    assert!(message.contains(&folder), "{message}");
    client.cancel().await?;
    Ok(())
}

// AC: CE-18.1
#[test]
fn stdout_is_only_json_rpc() -> TestResult {
    let (_dir, project) = fixture_copy()?;
    let config = tempfile::tempdir()?;
    let mut child = Command::new(env!("CARGO_BIN_EXE_theos-mcp"))
        .current_dir(&project)
        .env("THEOS_LOG", "trace")
        .env("THEOS_CONFIG_DIR", config.path())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()?;
    let mut stdin = child.stdin.take().ok_or("no stdin")?;
    let messages = [
        r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"test","version":"0"}}}"#,
        r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#,
        r#"{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"project_info","arguments":{}}}"#,
    ];
    for message in messages {
        writeln!(stdin, "{message}")?;
    }
    drop(stdin);
    let output = child.wait_with_output()?;
    assert_eq!(output.status.code(), Some(0));
    let stdout = String::from_utf8(output.stdout)?;
    let lines: Vec<&str> = stdout.lines().filter(|l| !l.trim().is_empty()).collect();
    assert!(lines.len() >= 2, "expected two responses: {stdout}");
    for line in lines {
        let value: serde_json::Value = serde_json::from_str(line)?;
        assert_eq!(value["jsonrpc"], "2.0", "{line}");
    }
    Ok(())
}
