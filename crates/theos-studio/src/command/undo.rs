//! Undo and redo (spec §5.7): an entry of the journal played back through the pipeline, so it
//! gets the same checks and the same verdict as the command did.

use std::collections::BTreeMap;

use super::journal::{EntryId, Scope};
use super::pipeline::{self, Outcome, Work};
use super::route::{Applied, HumanRoute};
use super::{CommandError, FileChange, Plan, project_path};
use crate::Workspace;

impl Workspace {
    /// Takes back the newest entry in `scope`: every file it changed gets its bytes from before
    /// the command. The replay is a command of its own, but adds no journal entry: the entry
    /// moves to the redo list.
    ///
    /// # Errors
    ///
    /// [`CommandError::NothingToUndo`] when no entry matches. [`CommandError::Changed`] when a
    /// file differs from what the command wrote. [`CommandError::Rejected`] when taking the
    /// command back would introduce an error, for example by removing a file that another
    /// record links. [`CommandError::Path`] when the path of [`Scope::Record`] breaks the rules
    /// of a command path. Otherwise as [`Workspace::run_with`]; the entry stays where it was.
    pub fn undo(&mut self, scope: Scope) -> Result<Outcome, CommandError> {
        let found = self.commands.journal.find_done(checked(scope)?);
        let (id, applied) = found.ok_or(CommandError::NothingToUndo)?;
        let work = replay("undo", id, applied, true);
        let outcome = pipeline::run(self, work, &mut HumanRoute)?;
        self.commands.journal.mark_undone(id);
        Ok(outcome)
    }

    /// Applies the most recently undone entry in `scope` again, with the bytes it wrote.
    ///
    /// # Errors
    ///
    /// [`CommandError::NothingToRedo`] when no entry matches; otherwise as [`Workspace::undo`].
    pub fn redo(&mut self, scope: Scope) -> Result<Outcome, CommandError> {
        let found = self.commands.journal.find_undone(checked(scope)?);
        let (id, applied) = found.ok_or(CommandError::NothingToRedo)?;
        let work = replay("redo", id, applied, false);
        let outcome = pipeline::run(self, work, &mut HumanRoute)?;
        self.commands.journal.mark_redone(id);
        Ok(outcome)
    }
}

/// `scope` with the path of [`Scope::Record`] read as every command path is: a `\` as `/`.
///
/// # Errors
///
/// [`CommandError::Path`] for a path outside the rules.
fn checked(scope: Scope) -> Result<Scope, CommandError> {
    match scope {
        Scope::Record(path) => Ok(Scope::Record(project_path(&path)?)),
        Scope::Newest => Ok(Scope::Newest),
    }
}

/// The replay of `applied`: reversed for an undo, as it was for a redo. A replay allows the
/// findings of the state it restores.
fn replay(verb: &str, id: EntryId, applied: &Applied, reverse: bool) -> Work<'static> {
    let label = format!("{verb}: {}", applied.label);
    let (changes, moved, allowance) = if reverse {
        let changes = applied
            .changes
            .iter()
            .map(|change| FileChange {
                path: change.path.clone(),
                before: change.after.clone(),
                after: change.before.clone(),
            })
            .collect();
        let moved: BTreeMap<String, String> = applied
            .moved
            .iter()
            .map(|(old, new)| (new.clone(), old.clone()))
            .collect();
        (changes, moved, applied.counts_before.clone())
    } else {
        (
            applied.changes.clone(),
            applied.moved.clone(),
            applied.counts_after.clone(),
        )
    };
    Work::Replay {
        label,
        plan: Plan { changes, moved },
        allowance,
        entry: id,
    }
}
