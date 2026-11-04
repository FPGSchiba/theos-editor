//! Smoke test: the `theos-editor` binary starts and exits successfully.

use std::io;
use std::process::Command;

// AC: CE-08.1
#[test]
fn theos_editor_binary_runs() -> io::Result<()> {
    let status = Command::new(env!("CARGO_BIN_EXE_theos-editor")).status()?;
    assert!(status.success(), "theos-editor exited with {status}");
    Ok(())
}
