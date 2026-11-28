//! What the app is: its version, the OS it runs on and whether the `engine` feature is on.

use serde::Serialize;

use super::timed;

/// The app's version, OS and features.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub struct AppInfo {
    /// The app's version.
    pub version: String,
    /// The OS: `windows`, `macos` or `linux`. The UI picks Ctrl or Cmd by it.
    pub os: String,
    /// Whether the app was built with the cargo feature `engine` (CE-32.3).
    pub engine: bool,
}

impl AppInfo {
    /// The running build's.
    #[must_use]
    pub fn current() -> AppInfo {
        AppInfo {
            version: env!("CARGO_PKG_VERSION").to_owned(),
            os: std::env::consts::OS.to_owned(),
            engine: cfg!(feature = "engine"),
        }
    }
}

/// The app's version, OS and features.
#[tauri::command]
#[specta::specta]
#[must_use]
pub fn app_info() -> AppInfo {
    timed("app_info", AppInfo::current)
}
