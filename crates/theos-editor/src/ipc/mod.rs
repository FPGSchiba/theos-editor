//! The IPC commands, one module per group, and their error.
//!
//! Each command is a thin map: it takes the open [`Workspace`](theos_studio::Workspace) from the
//! [`AppState`](crate::AppState), calls `theos-studio` and returns a view type of this crate.
//! A new command is a function in its group's module plus one line in
//! [`specta_builder`](crate::specta_builder).

pub mod app;
pub mod codex;
pub mod error;
pub mod project;
pub mod views;

use std::time::Instant;

use error::IpcError;

/// `count` as the `u32` the views carry; a larger count saturates.
pub(crate) fn saturate(count: usize) -> u32 {
    u32::try_from(count).unwrap_or(u32::MAX)
}

/// Runs the command `command`, which cannot fail, and logs its time at `debug`.
pub(crate) fn timed<T>(command: &'static str, run: impl FnOnce() -> T) -> T {
    let started = Instant::now();
    let value = run();
    let elapsed_ms = started.elapsed().as_secs_f64() * 1000.0;
    tracing::debug!(command, elapsed_ms, "command done");
    value
}

/// Logs a finished command: its time at `debug`, and its error, if any, at `error`.
pub(crate) fn logged<T>(
    command: &'static str,
    started: Instant,
    result: Result<T, IpcError>,
) -> Result<T, IpcError> {
    let elapsed_ms = started.elapsed().as_secs_f64() * 1000.0;
    match &result {
        Ok(_) => tracing::debug!(command, elapsed_ms, "command done"),
        Err(error) => {
            tracing::debug!(command, elapsed_ms, "command failed");
            tracing::error!(command, %error, "command failed");
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_timed_command_returns_its_value() {
        assert_eq!(timed("synthetic", || 7), 7);
    }

    #[test]
    fn counts_saturate() {
        assert_eq!(saturate(7), 7);
        assert_eq!(saturate(usize::MAX), u32::MAX);
    }
}
