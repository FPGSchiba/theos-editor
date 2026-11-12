//! What the command layer keeps between commands of one workspace.

use std::time::Duration;

use super::journal::Journal;
use crate::lock::COMMAND_WAIT;

/// The state of the command layer in a [`Workspace`](crate::Workspace).
#[derive(Debug)]
pub(crate) struct CommandState {
    /// The applied commands, for undo and redo.
    pub(crate) journal: Journal,
    /// Whether a command of this workspace holds the repo lock right now.
    pub(crate) locked: bool,
    /// How long a command waits for the repo lock.
    pub(crate) lock_wait: Duration,
}

impl Default for CommandState {
    fn default() -> Self {
        CommandState {
            journal: Journal::default(),
            locked: false,
            lock_wait: COMMAND_WAIT,
        }
    }
}
