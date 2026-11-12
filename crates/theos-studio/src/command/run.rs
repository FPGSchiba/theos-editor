//! The entry points of the command layer on a [`Workspace`]: reading a file with its base, and
//! running a command through the pipeline.

use std::time::Duration;

use super::pipeline::{self, Outcome, Work};
use super::route::{HumanRoute, Route};
use super::{Base, Command, CommandError, read_file};
use crate::Workspace;

impl Workspace {
    /// The bytes of the project file `path` and their base, `None` when the file is absent. A
    /// caller keeps the base of what it showed and hands it back with the command that changes
    /// the file.
    ///
    /// # Errors
    ///
    /// [`CommandError::Path`] for a path outside the rules, [`CommandError::Read`] when the file
    /// exists but cannot be read.
    pub fn read(&self, path: &str) -> Result<Option<(Vec<u8>, Base)>, CommandError> {
        let bytes = read_file(self.project().root(), path)?;
        Ok(bytes.map(|bytes| {
            let base = Base::of(&bytes);
            (bytes, base)
        }))
    }

    /// Runs `command` on the human route: behind the repo lock, and with a journal entry when it
    /// changes something.
    ///
    /// # Errors
    ///
    /// As [`Workspace::run_with`].
    pub fn run(&mut self, command: &dyn Command) -> Result<Outcome, CommandError> {
        self.run_with(command, &mut HumanRoute)
    }

    /// Runs `command` through the pipeline on `route`. Once the files are written, the route
    /// always finishes; a failed reindex after the write is no error, the outcome says it with
    /// [`Outcome::reindexed`] `false` (ruling R5).
    ///
    /// # Errors
    ///
    /// Before the write, an error leaves the files and the journal as they were, and the index
    /// cache too, except where the pipeline, before it plans, brought the cache in line with files
    /// changed outside the workspace:
    /// [`CommandError::Lock`] when the repo lock cannot be taken,
    /// [`CommandError::Path`] when a path breaks the rules, a plan path is not in normal form (it
    /// has a `\`, ruling R22; a rename also refuses a file name with one), or a change would alter
    /// which files are records where the verdict cannot see it (ruling R21): a change of a
    /// `.gitignore`, on Windows the replacement of a hidden file, or a path that the disk spells
    /// otherwise or that passes a symbolic link to a folder,
    /// [`CommandError::Stale`] when a file changed on disk since the caller or the plan read it,
    /// [`CommandError::NotARecord`] when a record it moves would not be read as a record at its
    /// new path (ruling R16),
    /// [`CommandError::Rejected`] when the command would introduce errors,
    /// [`CommandError::ReadOnly`] when a file the command would replace or remove is read-only
    /// (ruling R9; nothing is written),
    /// or the error of the command's plan or of the index.
    ///
    /// A failed write is [`CommandError::Write`], with every file put back, or
    /// [`CommandError::Partial`], which names the files that keep the new bytes. The pipeline
    /// still brings the index cache in line with the files (a failure of that is logged), and the
    /// journal is unchanged.
    ///
    /// After the write, the files are written: an error of the route's [`Route::finish`] is
    /// returned with the files changed.
    pub fn run_with(
        &mut self,
        command: &dyn Command,
        route: &mut dyn Route,
    ) -> Result<Outcome, CommandError> {
        pipeline::run(self, Work::Command(command), route)
    }

    /// How long a command waits for the repo lock; [`COMMAND_WAIT`](crate::lock::COMMAND_WAIT)
    /// unless set.
    pub fn set_lock_wait(&mut self, wait: Duration) {
        self.commands.lock_wait = wait;
    }
}
