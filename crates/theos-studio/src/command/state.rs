//! What the command layer keeps between commands of one workspace.

/// The state of the command layer in a [`Workspace`](crate::Workspace). Later tasks give it the
/// journal and the lock flag.
#[derive(Debug, Default)]
pub(crate) struct CommandState {}
