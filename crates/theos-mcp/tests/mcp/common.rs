//! Helpers of the MCP tests: fixture copies, a client on the binary, structured results.

use std::path::{Path, PathBuf};

use rmcp::model::CallToolResult;
use rmcp::service::RunningService;
use rmcp::transport::TokioChildProcess;
use rmcp::{RoleClient, ServiceExt};
use tempfile::TempDir;

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

/// Starts `theos-mcp` with `args` in `project` and connects a client to it. The settings
/// folder is a throwaway one, so the user's own settings never leak into a test.
pub async fn client(
    project: &Path,
    args: &[&str],
) -> Result<RunningService<RoleClient, ()>, Error> {
    let config = tempfile::tempdir()?.keep();
    let mut command = tokio::process::Command::new(env!("CARGO_BIN_EXE_theos-mcp"));
    command
        .args(args)
        .current_dir(project)
        .env("THEOS_CONFIG_DIR", config);
    Ok(().serve(TokioChildProcess::new(command)?).await?)
}

/// The structured content of a tool result.
pub fn structured(result: &CallToolResult) -> Result<serde_json::Value, Error> {
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
