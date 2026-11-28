//! Waiting for git while watching a [`Cancel`] flag.
//!
//! A thread waits for git and hands its exit status over a channel, so the run sees the exit at
//! once; between the hand-overs the run checks the flag every [`POLL`], so a cancel is seen
//! within that interval (P46 §5.3: within 50 ms).

use std::io;
use std::process::{Child, ExitStatus};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError};
use std::thread::Scope;
use std::time::Duration;

use crate::Cancel;
use crate::tree::Tree;

/// How often the flag is checked while git runs.
pub(crate) const POLL: Duration = Duration::from_millis(10);

/// How a run of git ended.
pub(crate) enum Waited {
    /// git exited on its own.
    Exited(ExitStatus),
    /// The flag was set; git and its tree were stopped.
    Cancelled,
}

/// Waits for `child` on a thread of `scope` and stops `tree` when `cancel` is set.
///
/// `tree` is dropped when the wait ends.
pub(crate) fn wait<'scope>(
    scope: &'scope Scope<'scope, '_>,
    mut child: Child,
    tree: Tree,
    cancel: &Cancel,
) -> io::Result<Waited> {
    let (sender, receiver) = mpsc::channel();
    scope.spawn(move || {
        // The receiver is gone only when the run already returned.
        let _ = sender.send(child.wait());
    });
    loop {
        match receiver.recv_timeout(POLL) {
            Ok(status) => return status.map(Waited::Exited),
            Err(RecvTimeoutError::Timeout) if cancel.is_cancelled() => break,
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => return Err(gone()),
        }
    }
    after_cancel(&receiver, tree)
}

/// The decision once a cancel is seen. git may have exited just now: a status that has arrived
/// is the answer, `Exited`, and nothing is stopped, as git's process id may be reused. Only a git
/// still running is stopped, and the answer is `Cancelled`, unless (Unix) it exited successfully
/// just before the stop: then its result stands.
fn after_cancel(receiver: &Receiver<io::Result<ExitStatus>>, tree: Tree) -> io::Result<Waited> {
    if let Ok(status) = receiver.try_recv() {
        return status.map(Waited::Exited);
    }
    tree.stop();
    let status = receiver.recv().map_err(|_| gone())??;
    // git may have exited successfully between the check and the stop: what it did is done, so
    // its result is reported, not a cancel (P47 ruling 2). Only Unix can tell: a git killed
    // there ends by a signal, while closing the job on Windows ends it with exit code 0.
    if cfg!(unix) && status.success() {
        return Ok(Waited::Exited(status));
    }
    Ok(Waited::Cancelled)
}

fn gone() -> io::Error {
    io::Error::other("the thread that waits for git ended without an answer")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command;

    #[test]
    fn a_status_that_arrived_before_the_cancel_is_kept() -> io::Result<()> {
        let status = Command::new("git")
            .arg("--version")
            .stdout(std::process::Stdio::null())
            .status()?;
        let (sender, receiver) = mpsc::channel();
        sender.send(Ok(status)).map_err(|_| gone())?;
        let mut command = Command::new("git");
        command.arg("--version").stdout(std::process::Stdio::null());
        let (tree, mut child) = Tree::spawn(&mut command)?;
        child.wait()?;
        let waited = after_cancel(&receiver, tree)?;
        assert!(matches!(waited, Waited::Exited(got) if got == status));
        Ok(())
    }

    // Unix only: closing the job on Windows ends git with exit code 0, so a success there cannot
    // be told from a kill and the run stays `Cancelled`.
    #[cfg(unix)]
    #[test]
    fn a_success_that_arrives_while_stopping_is_kept() -> io::Result<()> {
        let mut command = Command::new("git");
        command.arg("--version").stdout(std::process::Stdio::null());
        let (tree, mut child) = Tree::spawn(&mut command)?;
        let status = child.wait()?;
        assert!(status.success());
        let (sender, receiver) = mpsc::channel();
        let late = std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(100));
            let _ = sender.send(Ok(status));
        });
        // Nothing has arrived yet, so the stop path runs and then receives the success.
        let waited = after_cancel(&receiver, tree)?;
        assert!(matches!(waited, Waited::Exited(got) if got == status));
        late.join().map_err(|_| gone())?;
        Ok(())
    }

    #[test]
    fn the_flag_is_checked_at_least_every_20_ms() {
        // Bounds the 50 ms of P46 §5.3 with room for stopping the tree.
        assert!(POLL <= Duration::from_millis(20), "{POLL:?}");
    }
}
