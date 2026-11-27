//! Waiting for git while watching a [`Cancel`] flag.
//!
//! A thread waits for git and hands its exit status over a channel, so the run sees the exit at
//! once; between the hand-overs the run checks the flag every [`POLL`], so a cancel is seen
//! within that interval (P46 §5.3: within 50 ms).

use std::io;
use std::process::{Child, ExitStatus};
use std::sync::mpsc::{self, RecvTimeoutError};
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
    // git may have exited just now; it is not stopped then, as its process id may be reused.
    if receiver.try_recv().is_err() {
        tree.stop();
        receiver.recv().map_err(|_| gone())??;
    }
    Ok(Waited::Cancelled)
}

fn gone() -> io::Error {
    io::Error::other("the thread that waits for git ended without an answer")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_flag_is_checked_at_least_every_20_ms() {
        // Bounds the 50 ms of P46 §5.3 with room for stopping the tree.
        assert!(POLL <= Duration::from_millis(20), "{POLL:?}");
    }
}
