//! Smoke test: the `theos-mcp` binary starts and exits successfully.

use std::io;
use std::process::{Command, Stdio};

// AC: CE-08.1
#[test]
fn theos_mcp_binary_runs() -> io::Result<()> {
    let status = Command::new(env!("CARGO_BIN_EXE_theos-mcp"))
        .stdin(Stdio::null())
        .status()?;
    assert!(status.success(), "theos-mcp exited with {status}");
    Ok(())
}
