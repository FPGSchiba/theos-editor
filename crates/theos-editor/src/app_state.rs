//! The state the commands share: the open project and the state file.

use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard, PoisonError};
use std::time::Instant;

use theos_studio::Workspace;

use crate::state::StateFile;

/// The app's state. One project is open at a time.
pub struct AppState {
    /// The open project, if any.
    pub(crate) workspace: Mutex<Option<Workspace>>,
    /// When the index of the open project was last synced.
    pub(crate) last_sync: Mutex<Option<Instant>>,
    /// Where the app remembers the last project.
    pub(crate) state_file: StateFile,
    /// The folder named on the command line (`theos-editor <path>`), if any.
    pub(crate) argument: Option<PathBuf>,
}

impl AppState {
    /// No open project, and the state file in `data_folder` (none with `None`).
    #[must_use]
    pub fn new(data_folder: Option<PathBuf>) -> AppState {
        AppState {
            workspace: Mutex::new(None),
            last_sync: Mutex::new(None),
            state_file: StateFile::in_folder(data_folder),
            argument: None,
        }
    }

    /// The same state, with `argument` as the folder named on the command line.
    #[must_use]
    pub fn with_argument(self, argument: Option<PathBuf>) -> AppState {
        AppState { argument, ..self }
    }
}

/// Locks `mutex`. A command that panicked while holding it left no half-done change behind
/// (each one replaces the value whole), so a poisoned lock is taken as it is.
pub(crate) fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}
