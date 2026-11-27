//! How the binary starts and stops (CE-18.1).

use std::io::Write;
use std::process::{Command, Stdio};

use crate::common::TestResult;

fn spawn(config: &std::path::Path) -> std::io::Result<std::process::Child> {
    Command::new(env!("CARGO_BIN_EXE_theos-mcp"))
        .env("THEOS_CONFIG_DIR", config)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
}

// AC: CE-18.1
#[test]
fn a_client_that_breaks_the_handshake_ends_the_server_with_exit_code_2() -> TestResult {
    let config = tempfile::tempdir()?;
    let mut child = spawn(config.path())?;
    let mut stdin = child.stdin.take().ok_or("no stdin")?;
    // A notification where the handshake belongs is a protocol failure.
    writeln!(
        stdin,
        r#"{{"jsonrpc":"2.0","method":"notifications/initialized"}}"#
    )?;
    stdin.flush()?;
    drop(stdin);
    let output = child.wait_with_output()?;
    assert_eq!(output.status.code(), Some(2), "{output:?}");
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("theos-mcp stopped"),
        "{output:?}"
    );
    Ok(())
}

// AC: CE-18.1
#[test]
fn a_client_that_closes_stdin_at_once_is_a_normal_end() -> TestResult {
    let config = tempfile::tempdir()?;
    let mut child = spawn(config.path())?;
    drop(child.stdin.take());
    let output = child.wait_with_output()?;
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    Ok(())
}
