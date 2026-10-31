//! Smoke test: the `theos-mcp` binary starts and exits successfully.

use std::io;
use std::process::Command;

// AC: CE-08.1
#[test]
fn theos_mcp_binary_runs() -> io::Result<()> {
    let status = Command::new(env!("CARGO_BIN_EXE_theos-mcp")).status()?;
    assert!(status.success(), "theos-mcp exited with {status}");
    Ok(())
}
