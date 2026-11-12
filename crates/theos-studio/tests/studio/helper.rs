//! The helper process of the cross-process tests. The test binary runs itself with
//! `helper::helper --exact` and the task in `THEOS_STUDIO_HELPER`; it signals through files only.

use std::path::PathBuf;
use std::time::{Duration, Instant};

use theos_content::{Record, Span, TextEdit};
use theos_studio::{CommandError, Edit, Workspace, lock};

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

/// Appends the lines `<tag> 1` to `<tag> <count>` to the record `path`, one edit each; a stale
/// base is read again and retried.
pub fn append_lines(
    workspace: &mut Workspace,
    path: &str,
    tag: &str,
    count: usize,
) -> Result<(), Box<dyn std::error::Error>> {
    for n in 1..=count {
        loop {
            let (bytes, base) = workspace.read(path)?.ok_or("the record is missing")?;
            let record = Record::parse(&String::from_utf8(bytes)?)?;
            let end = record.text().len();
            let edit = Edit {
                path: path.to_owned(),
                base,
                edits: vec![TextEdit {
                    span: Span { start: end, end },
                    replacement: format!(
                        "{tag} {n}
"
                    ),
                }],
            };
            match workspace.run(&edit) {
                Ok(_) => break,
                Err(CommandError::Stale { .. }) => {}
                Err(other) => return Err(other.into()),
            }
        }
    }
    Ok(())
}

/// Runs the task of `THEOS_STUDIO_HELPER`; returns at once when it is unset.
#[test]
fn helper() -> Result<(), Box<dyn std::error::Error>> {
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
            workspace.set_lock_wait(Duration::from_secs(10));
            std::fs::write(&ready, "ready")?;
            append_lines(&mut workspace, &path, &tag, count)?;
        }
    }
    Ok(())
}
