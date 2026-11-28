//! `run`, which `main` calls, on Tauri's mock runtime: it starts logging, builds the app and
//! returns when its last window is gone.

use std::ffi::OsString;
use std::sync::{Arc, Mutex};

use tauri::test::{MockRuntime, mock_builder, mock_context, noop_assets};
use tauri::{Manager, RunEvent, WebviewUrl, WebviewWindowBuilder};

type TestResult = Result<(), Box<dyn std::error::Error>>;

/// What closing the window `main` gave, once the app ran.
type Closed = Arc<Mutex<Option<Result<(), String>>>>;

/// A plugin that closes the window `main` once the app runs, so `run` returns, and keeps how that
/// went in `closed`.
fn close_when_ready(closed: Closed) -> tauri::plugin::TauriPlugin<MockRuntime> {
    tauri::plugin::Builder::new("synthetic-close-when-ready")
        .on_event(move |app, event| {
            if matches!(event, RunEvent::Ready)
                && let Some(window) = app.get_webview_window("main")
                && let Ok(mut slot) = closed.lock()
            {
                *slot = Some(window.destroy().map_err(|error| error.to_string()));
            }
        })
        .build()
}

#[test]
fn run_starts_logging_and_returns_when_the_window_is_gone() -> TestResult {
    let closed = Closed::default();
    let builder = mock_builder()
        .plugin(close_when_ready(Arc::clone(&closed)))
        .setup(|app| {
            WebviewWindowBuilder::new(app, "main", WebviewUrl::default()).build()?;
            Ok(())
        });
    let args = [OsString::from("theos-editor"), OsString::from("synthetic")];
    theos_editor::run(builder, mock_context(noop_assets()), args)?;
    let closed = closed.lock().map_err(|error| error.to_string())?.clone();
    assert_eq!(closed, Some(Ok(())));
    assert!(
        !theos_editor::start_logging(),
        "run started logging, so it cannot start again"
    );
    Ok(())
}
