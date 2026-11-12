//! Smoke test: the `theos` binary starts and exits successfully (with `--help`: a bare run now needs a command).

use std::io;
use std::process::Command;

// AC: CE-08.1
#[test]
fn theos_binary_runs() -> io::Result<()> {
    let status = Command::new(env!("CARGO_BIN_EXE_theos"))
        .arg("--help")
        .status()?;
    assert!(status.success(), "theos exited with {status}");
    Ok(())
}
