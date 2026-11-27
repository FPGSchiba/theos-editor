//! `project_info` and the server's start-up behaviour (CE-18.1).

use std::io::Write;
use std::process::{Command, Stdio};

use rmcp::model::CallToolRequestParams;

use serde_json::{Value, json};
use theos_studio::project;

use crate::common::{
    TestResult, assert_error, client, copy_dir, fixture, fixture_copy, json, structured, studio,
};

/// The records of the `codex` fixture.
const FIXTURE_RECORDS: u64 = 18;

/// The parameters of every tool (spec 5.8): its properties, and which of them are required.
const TOOL_PARAMS: [(&str, &[&str], &[&str]); 6] = [
    ("project_info", &[], &[]),
    ("codex_search", &["query", "limit"], &["query"]),
    ("codex_read", &["id", "path"], &[]),
    (
        "codex_list",
        &["kind", "status", "folder", "offset", "limit"],
        &[],
    ),
    ("codex_graph", &["id", "depth", "direction"], &["id"]),
    ("ledger_check", &["id"], &[]),
];

// AC: CE-18.1
#[tokio::test]
async fn lists_exactly_the_six_tools() -> TestResult {
    let (_dir, project) = fixture_copy()?;
    let client = client(&project, &[]).await?;
    let tools = client.list_all_tools().await?;
    let mut names: Vec<&str> = tools.iter().map(|tool| &*tool.name).collect();
    names.sort_unstable();
    assert_eq!(
        names,
        [
            "codex_graph",
            "codex_list",
            "codex_read",
            "codex_search",
            "ledger_check",
            "project_info"
        ]
    );
    for tool in &tools {
        let name = &*tool.name;
        // G7: tool names are `^[a-z][a-z0-9_]*$`.
        assert!(name.starts_with(|c: char| c.is_ascii_lowercase()), "{name}");
        assert!(
            name.chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_'),
            "{name}"
        );
        assert!(tool.output_schema.is_some(), "{name}: no output schema");
        assert_description_says_read_only(name, tool.description.as_deref());
        assert_input_schema(name, &Value::Object((*tool.input_schema).clone()))?;
        let read_only = tool.annotations.as_ref().and_then(|a| a.read_only_hint);
        assert_eq!(read_only, Some(true), "{name}");
    }
    client.cancel().await?;
    Ok(())
}

fn assert_description_says_read_only(name: &str, description: Option<&str>) {
    let description = description.unwrap_or_default();
    assert!(description.contains("read-only"), "{name}: {description}");
}

/// Asserts that `schema` is the object schema of `name`'s parameters: exactly the expected
/// properties, the required keys, and no unknown key accepted.
fn assert_input_schema(name: &str, schema: &Value) -> TestResult {
    let (_, properties, required) = TOOL_PARAMS
        .iter()
        .find(|(tool, _, _)| *tool == name)
        .ok_or_else(|| format!("{name}: not a known tool"))?;
    assert_eq!(schema["type"], "object", "{name}: {schema}");
    let mut found: Vec<&str> = schema["properties"]
        .as_object()
        .map(|map| map.keys().map(String::as_str).collect())
        .unwrap_or_default();
    found.sort_unstable();
    let mut wanted = properties.to_vec();
    wanted.sort_unstable();
    assert_eq!(found, wanted, "{name}: {schema}");
    let mut found: Vec<&str> = schema["required"]
        .as_array()
        .map(|list| list.iter().filter_map(Value::as_str).collect())
        .unwrap_or_default();
    found.sort_unstable();
    assert_eq!(found, *required, "{name}: {schema}");
    assert_eq!(schema["additionalProperties"], false, "{name}: {schema}");
    Ok(())
}

// AC: CE-18.1
#[tokio::test]
async fn project_info_over_stdio() -> TestResult {
    let (_dir, project) = fixture_copy()?;
    let client = client(&project, &[]).await?;
    let tools = client.list_all_tools().await?;
    let tools: Vec<&str> = tools.iter().map(|tool| &*tool.name).collect();
    let result = client
        .call_tool(CallToolRequestParams::new("project_info"))
        .await?;
    assert_ne!(result.is_error, Some(true), "{result:?}");
    let value = structured(&result)?;
    assert_eq!(value["cache"], "file");
    assert_eq!(value["records"], FIXTURE_RECORDS);
    let ws = studio(&project)?;
    assert_eq!(value, json(&project::info(&ws, &tools)?)?);
    client.cancel().await?;
    Ok(())
}

// AC: CE-18.1
#[tokio::test]
async fn project_info_refuses_an_argument() -> TestResult {
    let (_dir, project) = fixture_copy()?;
    let client = client(&project, &[]).await?;
    let result = client.call("project_info", json!({ "bogus": 1 })).await?;
    assert_error(&result, "bogus")?;
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
    assert_error(&first, "no theos.toml")?;
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
    assert_error(&result, "theos.toml")?;
    assert_error(&result, &folder)?;
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
    let mut answered = false;
    for line in lines {
        let value: serde_json::Value = serde_json::from_str(line)?;
        assert_eq!(value["jsonrpc"], "2.0", "{line}");
        if value["id"] == 2 {
            assert!(value.get("result").is_some(), "no result: {line}");
            answered = true;
        }
    }
    assert!(answered, "no response with id 2: {stdout}");
    Ok(())
}
