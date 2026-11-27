//! Helpers of the MCP tests: fixture copies, a client on the binary, structured results.

use std::path::{Path, PathBuf};

use rmcp::model::{CallToolRequestParams, CallToolResult};
use rmcp::service::RunningService;
use rmcp::transport::TokioChildProcess;
use rmcp::{RoleClient, ServiceExt};
use serde_json::Value;
use tempfile::TempDir;
use theos_content::Index;
use theos_studio::Workspace;

/// The error of a test or a test helper.
pub type Error = Box<dyn std::error::Error>;

/// The result of a test.
pub type TestResult = Result<(), Error>;

/// The P05 fixture project `codex` of `theos-studio`.
pub fn fixture() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../theos-studio/tests/fixtures/projects/codex")
}

/// Copies the folder `from` into `to`, recursively.
pub fn copy_dir(from: &Path, to: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(to)?;
    for entry in std::fs::read_dir(from)? {
        let entry = entry?;
        let target = to.join(entry.file_name());
        // A cache left in the fixture folder by a local run is not part of the fixture.
        if entry.file_name() == "cache" && from.ends_with(".theos") {
            continue;
        }
        if entry.file_type()?.is_dir() {
            copy_dir(&entry.path(), &target)?;
        } else {
            std::fs::copy(entry.path(), target)?;
        }
    }
    Ok(())
}

/// A temp folder holding a copy of the `codex` fixture, and the project folder in it.
pub fn fixture_copy() -> Result<(TempDir, PathBuf), Error> {
    let dir = tempfile::tempdir()?;
    let project = dir.path().join("project");
    copy_dir(&fixture(), &project)?;
    Ok((dir, project))
}

/// A client on a running `theos-mcp`. It owns the throwaway settings folder of the server, so
/// the folder lives as long as the client and goes with it.
pub struct Client {
    service: RunningService<RoleClient, ()>,
    _config: TempDir,
}

impl std::ops::Deref for Client {
    type Target = RunningService<RoleClient, ()>;

    fn deref(&self) -> &Self::Target {
        &self.service
    }
}

impl Client {
    /// Calls the tool `name` with the JSON object `arguments`.
    pub async fn call(
        &self,
        name: &'static str,
        arguments: Value,
    ) -> Result<CallToolResult, Error> {
        let object = arguments.as_object().ok_or("arguments must be an object")?;
        let params = CallToolRequestParams::new(name).with_arguments(object.clone());
        Ok(self.service.call_tool(params).await?)
    }

    /// Calls the tool `name` and returns its structured content; an error result fails the test.
    pub async fn structured(&self, name: &'static str, arguments: Value) -> Result<Value, Error> {
        let result = self.call(name, arguments).await?;
        if result.is_error == Some(true) {
            return Err(format!("{name} returned an error: {result:?}").into());
        }
        structured(&result)
    }

    /// Stops the client and the server.
    pub async fn cancel(self) -> Result<(), Error> {
        self.service.cancel().await?;
        Ok(())
    }
}

/// Starts `theos-mcp` with `args` in `project` and connects a client to it. The settings
/// folder is a throwaway one, so the user's own settings never leak into a test.
pub async fn client(project: &Path, args: &[&str]) -> Result<Client, Error> {
    let config = tempfile::tempdir()?;
    let mut command = tokio::process::Command::new(env!("CARGO_BIN_EXE_theos-mcp"));
    command
        .args(args)
        .current_dir(project)
        .env("THEOS_CONFIG_DIR", config.path());
    let service = ().serve(TokioChildProcess::new(command)?).await?;
    Ok(Client {
        service,
        _config: config,
    })
}

/// A studio workspace on `project`, with its index synced: what the server computes from.
pub fn studio(project: &Path) -> Result<Workspace, Error> {
    let mut workspace = Workspace::open(project)?;
    workspace.write(Index::sync)?;
    Ok(workspace)
}

/// The JSON of a studio view.
pub fn json<T: serde::Serialize>(view: &T) -> Result<Value, Error> {
    Ok(serde_json::to_value(view)?)
}

/// The structured content of a tool result.
pub fn structured(result: &CallToolResult) -> Result<Value, Error> {
    result
        .structured_content
        .clone()
        .ok_or_else(|| "the result has no structured content".into())
}

/// The text of the first content block of a tool result.
pub fn text(result: &CallToolResult) -> Result<String, Error> {
    let block = result.content.first().ok_or("the result has no content")?;
    Ok(block
        .as_text()
        .ok_or("the first block is not text")?
        .text
        .clone())
}

/// Asserts that `result` is an error result: `isError`, exactly one text block, and that
/// block holds `needle`.
pub fn assert_error(result: &CallToolResult, needle: &str) -> TestResult {
    assert_eq!(result.is_error, Some(true), "{result:?}");
    assert_eq!(result.content.len(), 1, "{result:?}");
    let message = text(result)?;
    assert!(message.contains(needle), "`{needle}` not in: {message}");
    Ok(())
}
