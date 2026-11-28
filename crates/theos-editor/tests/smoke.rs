//! Smoke test: the `theos-editor` binary is built, and the app it runs builds.
//!
//! The binary opens a window and runs until it is closed, so the test does not start it. The app
//! is built on Tauri's mock runtime instead; `app.rs` calls its commands.

use tauri::test::{mock_builder, mock_context, noop_assets};

// AC: CE-08.1
#[test]
fn theos_editor_binary_runs() -> tauri::Result<()> {
    let binary = std::path::Path::new(env!("CARGO_BIN_EXE_theos-editor"));
    assert!(binary.is_file(), "{} is not built", binary.display());
    theos_editor::builder(mock_builder(), None).build(mock_context(noop_assets()))?;
    Ok(())
}
