//! Windows: git runs in a job with kill-on-close; closing the job ends every process in it.

use std::io;
use std::os::windows::io::AsRawHandle;
use std::process::{Child, Command};

use win32job::Job;

/// The job that holds git and every process git starts; none when git exited before it could
/// join one.
pub(crate) struct Tree {
    job: Option<Job>,
}

impl Tree {
    /// Makes a job that ends its processes when it is closed, starts git and puts it into the
    /// job at once; every process git starts afterwards joins the job too.
    ///
    /// When git cannot be put into the job, see [`enter`].
    pub(crate) fn spawn(command: &mut Command) -> io::Result<(Tree, Child)> {
        let job = Job::create()?;
        let mut info = job.query_extended_limit_info()?;
        info.limit_kill_on_job_close();
        job.set_extended_limit_info(&info)?;
        let mut child = command.spawn()?;
        let job = enter(job, &mut child)?;
        Ok((Tree { job }, child))
    }

    /// Closes the job, which ends git and every process in it. git is waited for elsewhere.
    pub(crate) fn stop(self) {
        drop(self.job);
    }
}

/// Puts `child` into `job` and returns the job, or none when `child` has already exited.
///
/// The assignment races git's exit: a fast git (`git --version`) can exit before this thread
/// gets to run again, and Windows refuses to put an exited process into a job ("Access is
/// denied"). Such a run goes on without a job: no tree is left to stop, and git's exit status
/// and output are collected as usual.
fn enter(job: Job, child: &mut Child) -> io::Result<Option<Job>> {
    match job.assign_process(child.as_raw_handle() as isize) {
        Ok(()) => Ok(Some(job)),
        Err(error) => {
            assignment_failed(child, error.into())?;
            Ok(None)
        }
    }
}

/// Handles a failed assignment of `child` to its job: fine when `child` has exited; otherwise
/// `child` runs outside the job, so it is killed and `error` returned.
fn assignment_failed(child: &mut Child, error: io::Error) -> io::Result<()> {
    if let Ok(Some(_)) = child.try_wait() {
        return Ok(());
    }
    // git runs outside the job: end it as well as std can, then report.
    let _ = child.kill();
    let _ = child.wait();
    Err(error)
}

#[cfg(test)]
mod tests {
    use std::io::Read;
    use std::process::Stdio;

    use super::*;

    fn git(args: &[&str]) -> io::Result<Child> {
        Command::new("git")
            .args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
    }

    #[test]
    fn git_that_exited_before_the_assignment_keeps_its_result() -> io::Result<()> {
        // The race of a fast `git --version` on a loaded machine, made certain: git has exited
        // before it is put into the job, which Windows refuses.
        let mut child = git(&["--version"])?;
        let status = child.wait()?;
        let job = enter(Job::create()?, &mut child)?;
        assert!(job.is_none(), "an exited process cannot join a job");
        let mut stdout = String::new();
        child
            .stdout
            .take()
            .ok_or_else(|| io::Error::other("no stdout"))?
            .read_to_string(&mut stdout)?;
        assert!(status.success(), "{status:?}");
        assert!(stdout.starts_with("git version "), "{stdout:?}");
        Ok(())
    }

    #[test]
    fn a_failed_assignment_while_git_runs_kills_git_and_is_an_error() -> io::Result<()> {
        // `hash-object --stdin` waits for its input, so git is still running here.
        let mut child = git(&["hash-object", "--stdin"])?;
        let refused = io::Error::from(io::ErrorKind::PermissionDenied);
        let result = assignment_failed(&mut child, refused);
        assert_eq!(
            result.err().map(|error| error.kind()),
            Some(io::ErrorKind::PermissionDenied)
        );
        let status = child.try_wait()?;
        assert!(status.is_some_and(|status| !status.success()), "{status:?}");
        Ok(())
    }
}
