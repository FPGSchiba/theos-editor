//! The helper process of the cross-process tests. The test binary runs itself with
//! `helper::helper --exact` and the task in `THEOS_STUDIO_HELPER`; it signals through files only.

use std::path::PathBuf;
use std::time::{Duration, Instant};

use theos_studio::lock;

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
    }
    Ok(())
}
