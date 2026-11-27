//! Windows: git runs in a job with kill-on-close; closing the job ends every process in it.

use std::io;
use std::os::windows::io::AsRawHandle;
use std::process::{Child, Command};

use win32job::Job;

/// The job that holds git and every process git starts.
pub(crate) struct Tree {
    job: Job,
}

impl Tree {
    /// Makes a job that ends its processes when it is closed, starts git and puts it into the
    /// job at once; every process git starts afterwards joins the job too.
    ///
    /// When git cannot be put into the job, git is killed and the error returned.
    pub(crate) fn spawn(command: &mut Command) -> io::Result<(Tree, Child)> {
        let job = Job::create()?;
        let mut info = job.query_extended_limit_info()?;
        info.limit_kill_on_job_close();
        job.set_extended_limit_info(&info)?;
        let mut child = command.spawn()?;
        if let Err(error) = job.assign_process(child.as_raw_handle() as isize) {
            // git runs outside the job: end it as well as std can, then report.
            let _ = child.kill();
            let _ = child.wait();
            return Err(error.into());
        }
        Ok((Tree { job }, child))
    }

    /// Closes the job, which ends git and every process in it. git is waited for elsewhere.
    pub(crate) fn stop(self) {
        drop(self.job);
    }
}
