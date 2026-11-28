//! Unix: git runs in its own process group; a `SIGKILL` to the group ends every process in it.

use std::io;
use std::os::unix::process::CommandExt;
use std::process::{Child, Command};

use nix::errno::Errno;
use nix::sys::signal::{Signal, killpg};
use nix::unistd::Pid;

/// git's process group, whose id is git's process id.
pub(crate) struct Tree {
    group: Pid,
}

impl Tree {
    /// Starts git in a new process group of its own.
    pub(crate) fn spawn(command: &mut Command) -> io::Result<(Tree, Child)> {
        let mut child = command.process_group(0).spawn()?;
        match i32::try_from(child.id()) {
            Ok(group) => Ok((
                Tree {
                    group: Pid::from_raw(group),
                },
                child,
            )),
            Err(error) => {
                // Not reached: a process id fits `pid_t`. git cannot be stopped as a group.
                let _ = child.kill();
                let _ = child.wait();
                Err(io::Error::other(error))
            }
        }
    }

    /// Sends `SIGKILL` to git's process group. git is waited for elsewhere.
    pub(crate) fn stop(self) {
        match killpg(self.group, Signal::SIGKILL) {
            // No process is left in the group: nothing to stop.
            Ok(()) | Err(Errno::ESRCH) => {}
            Err(error) => {
                tracing::warn!(%error, group = %self.group, "could not stop git's process group");
            }
        }
    }
}
