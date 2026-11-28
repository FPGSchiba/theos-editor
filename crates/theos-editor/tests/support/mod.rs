//! Shared setup of the command tests: the app on Tauri's mock runtime, IPC calls, fixtures.

use std::path::Path;

use serde_json::Value;
use tauri::ipc::{CallbackFn, InvokeBody};
use tauri::test::{
    INVOKE_KEY, MockRuntime, get_ipc_response, mock_builder, mock_context, noop_assets,
};
use tauri::webview::InvokeRequest;
use tauri::{App, WebviewUrl, WebviewWindow, WebviewWindowBuilder};
use theos_editor::AppState;

/// The error of a test.
pub type TestResult = Result<(), Box<dyn std::error::Error>>;

/// The fixture projects, which `theos-studio` holds so its tests and these share them.
pub fn fixtures() -> &'static Path {
    Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../theos-studio/tests/fixtures/projects"
    ))
}

/// The app and its window `main`.
pub type TestApp = tauri::Result<(App<MockRuntime>, WebviewWindow<MockRuntime>)>;

/// The app without a state file, and its window `main`.
pub fn app() -> TestApp {
    app_with(AppState::new(None))
}

/// The app with `state`, and its window `main`.
pub fn app_with(state: AppState) -> TestApp {
    window(theos_editor::builder_with(mock_builder(), state))
}

/// The app that [`theos_editor::builder`] sets up, and its window `main`.
pub fn default_app() -> TestApp {
    window(theos_editor::builder(mock_builder(), None))
}

fn window(builder: tauri::Builder<MockRuntime>) -> TestApp {
    log_everything();
    let app = builder.build(mock_context(noop_assets()))?;
    let window = WebviewWindowBuilder::new(&app, "main", WebviewUrl::default()).build()?;
    Ok((app, window))
}

/// Logs every level to the test's output, so the tests run the code that builds the log lines.
/// Whether it started: `false` when an earlier test of the process started it.
fn log_everything() -> bool {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::TRACE)
        .with_test_writer()
        .try_init()
        .is_ok()
}

/// Calls `cmd` with `args` through IPC: the answer, or the error as JSON. A failure of the call
/// itself is an error too, as a JSON string.
pub fn call(window: &WebviewWindow<MockRuntime>, cmd: &str, args: Value) -> Result<Value, Value> {
    let origin = if cfg!(windows) {
        "http://tauri.localhost"
    } else {
        "tauri://localhost"
    };
    let request = InvokeRequest {
        cmd: cmd.into(),
        callback: CallbackFn(0),
        error: CallbackFn(1),
        url: tauri::Url::parse(origin).map_err(|error| Value::String(error.to_string()))?,
        body: InvokeBody::Json(args),
        headers: tauri::http::HeaderMap::default(),
        invoke_key: INVOKE_KEY.to_string(),
    };
    let body = get_ipc_response(window, request)?;
    body.deserialize::<Value>()
        .map_err(|error| Value::String(error.to_string()))
}

/// The folder of a project's index cache, which a copy leaves out.
const CACHE: &str = ".theos";

/// A copy of the fixture project `name` in `into`, so tests never write into the fixtures. A
/// `.theos/` folder (a cache a manual start left in the fixture) is not copied.
pub fn copy_fixture(name: &str, into: &Path) -> std::io::Result<()> {
    copy_dir(&fixtures().join(name), into)
}

fn copy_dir(from: &Path, to: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(to)?;
    for entry in std::fs::read_dir(from)? {
        let entry = entry?;
        if entry.file_name() == CACHE {
            continue;
        }
        let target = to.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_dir(&entry.path(), &target)?;
        } else {
            std::fs::copy(entry.path(), &target)?;
        }
    }
    Ok(())
}
