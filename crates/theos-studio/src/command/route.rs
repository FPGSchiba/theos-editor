//! Routes (spec §5.10): the seam between the pipeline, which is the same for every write, and
//! what happens around it.
//!
//! A route decides whether the command holds the repo lock and what follows the write. P06's
//! [`HumanRoute`] holds the lock and adds a journal entry; P07's agent route commits on its
//! session branch instead, with no change to the pipeline or the commands.

use std::collections::BTreeMap;

use super::journal::EntryId;
use super::{CommandError, FileChange};
use crate::Workspace;
use crate::introduced::FindingCounts;

/// A command the pipeline wrote and reindexed, as it hands it to the route.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Applied {
    /// The command's label.
    pub label: String,
    /// Every file it changed, with its exact bytes before and after, in plan order.
    pub changes: Vec<FileChange>,
    /// Old path to new path, for the files the command moved.
    pub moved: BTreeMap<String, String>,
    /// The findings of the affected paths before the command, counted under the paths they had
    /// then (not mapped through the files the command moved).
    pub counts_before: FindingCounts,
    /// The findings of the affected paths after the command.
    pub counts_after: FindingCounts,
}

/// What happens around the pipeline of a command.
pub trait Route {
    /// Whether the route holds the repo lock for the whole command.
    fn holds_lock(&self) -> bool;

    /// After the write and the reindex: does what the route does with an applied command, and
    /// returns the journal entry it added, if it keeps a journal. It runs after every successful
    /// write, also when the reindex failed; the outcome reports that with
    /// [`Outcome::reindexed`](super::Outcome::reindexed) `false`, never as an error.
    ///
    /// # Errors
    ///
    /// A [`CommandError`] when the route cannot finish. The files are written then, and the
    /// command returns this error with the files changed. An error before the write, and a
    /// failed write, never reach `finish`; [`Workspace::run_with`] says what they leave behind.
    fn finish(
        &mut self,
        workspace: &mut Workspace,
        applied: Applied,
    ) -> Result<Option<EntryId>, CommandError>;
}

/// The route of a person at the editor: it holds the repo lock for the whole command, and each
/// applied command becomes a journal entry, which clears the redo list.
#[derive(Clone, Copy, Debug, Default)]
pub struct HumanRoute;

impl Route for HumanRoute {
    fn holds_lock(&self) -> bool {
        true
    }

    fn finish(
        &mut self,
        workspace: &mut Workspace,
        applied: Applied,
    ) -> Result<Option<EntryId>, CommandError> {
        Ok(Some(workspace.commands.journal.push(applied)))
    }
}
