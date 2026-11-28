//! The repo lock held by a workspace outside commands too: [`Workspace::hold`].

use std::ops::{Deref, DerefMut};
use std::path::Path;

use crate::command::state::CommandState;
use crate::lock::{self, LockError, LockGuard};
use crate::workspace::Workspace;

/// A workspace under the repo lock, when it was asked to take it. Dropping it, on every way out,
/// a panic included, clears the workspace's flag and releases the lock. It stands for the
/// workspace: dereference it to keep working under the lock.
pub(crate) struct Holding<'w> {
    workspace: &'w mut Workspace,
    guard: Option<LockGuard>,
}

impl<'w> Holding<'w> {
    /// Takes the repo lock for `purpose` when `lock` is set; without it the workspace is passed
    /// through unlocked.
    ///
    /// # Errors
    ///
    /// [`LockError::Reentrant`] at once when this workspace holds the lock already; otherwise the
    /// errors of [`lock::acquire`].
    pub(crate) fn take(
        workspace: &'w mut Workspace,
        purpose: &str,
        lock: bool,
    ) -> Result<Holding<'w>, LockError> {
        let guard = if lock {
            let root = workspace.project().root().to_path_buf();
            Some(take_lock(&mut workspace.commands, &root, purpose)?)
        } else {
            None
        };
        Ok(Holding { workspace, guard })
    }
}

impl Deref for Holding<'_> {
    type Target = Workspace;

    fn deref(&self) -> &Workspace {
        self.workspace
    }
}

impl DerefMut for Holding<'_> {
    fn deref_mut(&mut self) -> &mut Workspace {
        self.workspace
    }
}

impl Drop for Holding<'_> {
    fn drop(&mut self) {
        if let Some(guard) = self.guard.take() {
            self.workspace.commands.locked = false;
            drop(guard);
        }
    }
}

impl Workspace {
    /// Takes the repo lock of the project for `purpose`, waiting as long as the workspace's lock
    /// wait. The returned guard stands for the workspace; dropping it releases the lock.
    ///
    /// # Errors
    ///
    /// [`LockError::Reentrant`] at once when this workspace holds the lock already,
    /// [`LockError::Busy`] after the wait, [`LockError::Io`] when the lock file cannot be used.
    pub(crate) fn hold(&mut self, purpose: &str) -> Result<Holding<'_>, LockError> {
        Holding::take(self, purpose, true)
    }
}

/// The repo lock of `root`, unless this workspace holds it already: then [`LockError::Reentrant`]
/// at once, instead of waiting for itself.
fn take_lock(state: &mut CommandState, root: &Path, label: &str) -> Result<LockGuard, LockError> {
    if state.locked {
        return Err(LockError::Reentrant);
    }
    let guard = lock::acquire(root, label, state.lock_wait)?;
    state.locked = true;
    Ok(guard)
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;
    use crate::test_support::{Fixture, TestResult};

    #[test]
    fn reentry_is_refused() -> TestResult {
        let dir = tempfile::tempdir()?;
        let mut state = CommandState {
            locked: true,
            ..CommandState::default()
        };
        let result = take_lock(&mut state, dir.path(), "again");
        assert!(matches!(result, Err(LockError::Reentrant)), "{result:?}");
        assert!(state.locked);
        // The refused call did not take the lock.
        drop(lock::acquire(dir.path(), "probe", Duration::ZERO)?);
        Ok(())
    }

    #[test]
    fn hold_sets_the_flag_and_the_guard_clears_it() -> TestResult {
        let mut fixture = Fixture::new(&[])?;
        let root = fixture.root().to_path_buf();
        {
            let mut held = fixture.workspace_mut().hold("synthetic")?;
            assert!(held.commands.locked);
            assert!(matches!(
                lock::acquire(&root, "probe", Duration::ZERO),
                Err(LockError::Busy { .. })
            ));
            assert!(matches!(held.hold("again"), Err(LockError::Reentrant)));
        }
        assert!(!fixture.workspace_mut().commands.locked);
        drop(lock::acquire(&root, "probe", Duration::ZERO)?);
        Ok(())
    }

    #[test]
    fn without_the_lock_the_workspace_passes_through() -> TestResult {
        let mut fixture = Fixture::new(&[])?;
        let root = fixture.root().to_path_buf();
        let held = Holding::take(fixture.workspace_mut(), "synthetic", false)?;
        assert!(!held.commands.locked);
        drop(lock::acquire(&root, "probe", Duration::ZERO)?);
        Ok(())
    }
}
