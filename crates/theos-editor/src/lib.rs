//! The authoring shell (Tauri 2). Its commands are thin maps onto `theos-studio`; their types are
//! exported for the UI through tauri-specta.
//!
//! [`builder`] sets up the app on any Tauri runtime, so the tests run the same app on Tauri's mock
//! runtime. The cargo feature `engine` is reserved for the engine part and stays empty until then
//! ([`presets`]).

pub mod app_state;
pub mod ipc;
pub mod menu;
pub mod presets;
pub mod state;

pub use app_state::AppState;
pub use ipc::error::IpcError;

use std::ffi::OsString;
use std::path::PathBuf;

use tauri::Runtime;
use tauri_specta::{ErrorHandlingMode, collect_commands};

/// Every command of the app, with the types they take and return.
///
/// Serde phases are off: the view types are only sent to the UI, so one type per name is exact,
/// where the phases would add a `_Deserialize` twin with optional keys for every `Option` field.
#[must_use]
pub fn specta_builder<R: Runtime>() -> tauri_specta::Builder<R> {
    tauri_specta::Builder::<R>::new()
        .error_handling(ErrorHandlingMode::Result)
        .disable_serde_phases()
        .commands(collect_commands![
            ipc::app::app_info,
            ipc::project::project_open,
            ipc::project::project_last,
            ipc::project::project_argument,
            ipc::project::project_sync,
            ipc::codex::codex_tabs,
            ipc::codex::codex_tree,
            ipc::codex::record_view,
            ipc::codex::codex_search,
        ])
}

/// Sets up the app on `builder`: the state, with the state file in [`state::folder`] and
/// `argument` as the folder named on the command line, the folder dialog and the commands.
#[must_use]
pub fn builder<R: Runtime>(
    builder: tauri::Builder<R>,
    argument: Option<PathBuf>,
) -> tauri::Builder<R> {
    builder_with(
        builder,
        AppState::new(state::folder()).with_argument(argument),
    )
}

/// Runs the editor until its window closes: logging, then the app [`builder`] sets up on
/// `builder`, with the folder named in `args` ([`argument`]). `main` calls it with the real
/// runtime, the tests with the mock one.
///
/// # Errors
///
/// When the app cannot be built.
pub fn run<R: Runtime>(
    builder: tauri::Builder<R>,
    context: tauri::Context<R>,
    args: impl IntoIterator<Item = OsString>,
) -> tauri::Result<()> {
    let started = start_logging();
    tracing::debug!(started, "logging");
    self::builder(builder, argument(args)).run(context)
}

/// Starts logging to stderr, filtered by `RUST_LOG`. Whether it started: `false` when logging
/// was started before.
#[must_use]
pub fn start_logging() -> bool {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .try_init()
        .is_ok()
}

/// The folder named on the command line, `theos-editor <path>`: the first of `args` after the
/// program's own name.
#[must_use]
pub fn argument(args: impl IntoIterator<Item = OsString>) -> Option<PathBuf> {
    args.into_iter().nth(1).map(PathBuf::from)
}

/// Sets up the app on `builder` with the given state: the folder dialog and the commands.
#[must_use]
pub fn builder_with<R: Runtime>(builder: tauri::Builder<R>, state: AppState) -> tauri::Builder<R> {
    builder
        .plugin(tauri_plugin_dialog::init())
        .manage(state)
        .invoke_handler(specta_builder::<R>().invoke_handler())
}
