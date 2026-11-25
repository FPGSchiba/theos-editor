//! The helper process of the cross-process tests. The test binary runs itself with
//! `helper::helper --exact` and the task in `THEOS_STUDIO_HELPER`; it signals through files only.

use std::path::PathBuf;
use std::time::{Duration, Instant};

use theos_studio::{CommandError, Workspace, lock};

use crate::common::{TestResult, append};

/// What the helper process does.
#[derive(serde::Serialize, serde::Deserialize)]
pub enum HelperTask {
    /// Takes the repo lock, writes the file `ready`, waits until the file `release` exists, then
    /// drops the lock.
    Hold {
        /// The project folder.
        root: PathBuf,
        /// The purpose the owner file names.
        purpose: String,
        /// Written once the lock is held.
        ready: PathBuf,
        /// The helper lets go when this file exists.
        release: PathBuf,
    },
    /// Opens a workspace, writes the file `ready`, then appends `count` lines `<tag> <n>` to the
    /// record `path`, each as one edit.
    Append {
        /// The project folder.
        root: PathBuf,
        /// Project-relative path of the record.
        path: String,
        /// The first word of every appended line.
        tag: String,
        /// How many lines to append.
        count: usize,
        /// Written once the workspace is open.
        ready: PathBuf,
    },
}

/// How long each of the two writers waits for the repo lock. The holder of the lock takes it
/// again a moment after it lets go, while a waiter pauses up to 200 ms between tries, so one
/// writer may run all its appends before the other gets a turn.
pub const WRITER_WAIT: Duration = Duration::from_secs(120);

/// How often one append is tried again after a stale base before the writer gives up, so that a
/// failure never turns into a hang.
const STALE_TRIES: usize = 1_000;

/// Appends the lines `<tag> 1` to `<tag> <count>` to the record `path`, one edit each.
pub fn append_lines(workspace: &mut Workspace, path: &str, tag: &str, count: usize) -> TestResult {
    for n in 1..=count {
        append_line(workspace, path, &format!("{tag} {n}\n"))?;
    }
    Ok(())
}

/// Appends `line` to the record `path` as one edit; a stale base is read again and retried, up to
/// `STALE_TRIES` times.
fn append_line(workspace: &mut Workspace, path: &str, line: &str) -> TestResult {
    for _ in 0..STALE_TRIES {
        let edit = append(workspace, path, line)?;
        match workspace.run(&edit) {
            Ok(_) => return Ok(()),
            Err(CommandError::Stale { .. }) => {}
            Err(other) => return Err(other.into()),
        }
    }
    Err(format!("appending {line:?} found a stale base {STALE_TRIES} times in a row").into())
}

/// Runs the task of `THEOS_STUDIO_HELPER`; returns at once when it is unset.
#[test]
fn helper() -> TestResult {
    let Ok(task) = std::env::var("THEOS_STUDIO_HELPER") else {
        return Ok(());
    };
    match serde_json::from_str::<HelperTask>(&task)? {
        HelperTask::Hold {
            root,
            purpose,
            ready,
            release,
        } => {
            let guard = lock::acquire(&root, &purpose, Duration::from_secs(30))?;
            std::fs::write(&ready, "ready")?;
            let start = Instant::now();
            while !release.exists() {
                if start.elapsed() > Duration::from_secs(30) {
                    return Err("timed out waiting for the release file".into());
                }
                std::thread::sleep(Duration::from_millis(10));
            }
            drop(guard);
        }
        HelperTask::Append {
            root,
            path,
            tag,
            count,
            ready,
        } => {
            let mut workspace = Workspace::open(&root)?;
            workspace.set_lock_wait(WRITER_WAIT);
            std::fs::write(&ready, "ready")?;
            append_lines(&mut workspace, &path, &tag, count)?;
        }
    }
    Ok(())
}
