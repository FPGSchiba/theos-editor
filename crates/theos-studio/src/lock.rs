//! The repo lock: one process at a time changes a project (spec P06 §5.9).
//!
//! The lock is an OS file lock on `<project_root>/.theos/cache/lock`, taken with
//! [`File::try_lock`]. The OS frees it when the holder exits or is killed, so it never goes
//! stale. Who holds it is told by the separate file `lock.owner`, because Windows does not let
//! another process read a locked file.

use std::fs::{File, TryLockError};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

/// How long a command waits for the lock.
pub const COMMAND_WAIT: Duration = Duration::from_secs(10);

/// The first pause between two tries.
const FIRST_PAUSE: Duration = Duration::from_millis(10);
/// The longest pause between two tries.
const LONGEST_PAUSE: Duration = Duration::from_millis(200);

/// Who holds the lock, as the owner file says.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Owner {
    /// The process id of the holder.
    pub pid: u32,
    /// The program of the holder: the file stem of its executable.
    pub program: String,
    /// What the holder is doing.
    pub purpose: String,
    /// When the holder took the lock, in seconds since the Unix epoch.
    pub since: u64,
}

/// Why the lock could not be taken.
#[derive(Debug, thiserror::Error)]
pub enum LockError {
    /// Another process held the lock for the whole wait.
    #[error("{}", busy_message(.owner.as_ref()))]
    Busy {
        /// The holder, when the owner file names one.
        owner: Option<Owner>,
        /// How long this call waited.
        waited: Duration,
    },
    /// The workspace that asks already holds the lock.
    #[error("this workspace already holds the repo lock")]
    Reentrant,
    /// The lock file or its folder could not be used.
    #[error("cannot use the repo lock {}: {source}", .path.display())]
    Io {
        /// The file or folder that failed.
        path: PathBuf,
        /// The cause.
        source: std::io::Error,
    },
}

/// The message of [`LockError::Busy`].
fn busy_message(owner: Option<&Owner>) -> String {
    match owner {
        Some(owner) => format!(
            "the project is busy: {} (pid {}) has been running `{}` for {} s",
            owner.program,
            owner.pid,
            owner.purpose,
            now_secs().saturating_sub(owner.since)
        ),
        None => "the project is busy: another theos process holds .theos/cache/lock".to_owned(),
    }
}

/// Holds the lock; dropping it deletes the owner file, then unlocks.
#[derive(Debug)]
pub struct LockGuard {
    file: File,
    owner_path: PathBuf,
}

impl Drop for LockGuard {
    fn drop(&mut self) {
        if let Err(error) = std::fs::remove_file(&self.owner_path)
            && error.kind() != std::io::ErrorKind::NotFound
        {
            tracing::warn!(path = %self.owner_path.display(), %error, "cannot delete the owner file");
        }
        if let Err(error) = self.file.unlock() {
            tracing::warn!(%error, "cannot unlock the repo lock");
        }
    }
}

/// Takes the repo lock `<project_root>/.theos/cache/lock` (folder and file created when
/// missing), trying again with a pause of 10 ms doubling up to 200 ms until `wait` has passed.
/// Writes `.theos/cache/lock.owner` after taking it.
///
/// # Errors
///
/// [`LockError::Busy`] when another process holds the lock for the whole `wait`;
/// [`LockError::Io`] when the folder, the lock file or the owner file cannot be used.
pub fn acquire(project_root: &Path, purpose: &str, wait: Duration) -> Result<LockGuard, LockError> {
    let dir = project_root.join(".theos").join("cache");
    let io = |path: &Path| {
        let path = path.to_path_buf();
        move |source| LockError::Io { path, source }
    };
    std::fs::create_dir_all(&dir).map_err(io(&dir))?;
    let lock_path = dir.join("lock");
    let owner_path = dir.join("lock.owner");
    let file = File::options()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(&lock_path)
        .map_err(io(&lock_path))?;

    let start = Instant::now();
    let mut pause = FIRST_PAUSE;
    loop {
        match file.try_lock() {
            Ok(()) => break,
            Err(TryLockError::Error(source)) => return Err(io(&lock_path)(source)),
            Err(TryLockError::WouldBlock) => {
                let waited = start.elapsed();
                if waited >= wait {
                    return Err(LockError::Busy {
                        owner: read_owner(&owner_path),
                        waited,
                    });
                }
                std::thread::sleep(pause.min(wait.saturating_sub(waited)));
                pause = (pause * 2).min(LONGEST_PAUSE);
            }
        }
    }

    // From here the guard unlocks again if writing the owner file fails.
    let guard = LockGuard { file, owner_path };
    let owner = Owner {
        pid: std::process::id(),
        program: program_name(),
        purpose: purpose.to_owned(),
        since: now_secs(),
    };
    let text = serde_json::to_string(&owner).map_err(|e| LockError::Io {
        path: guard.owner_path.clone(),
        source: e.into(),
    })?;
    std::fs::write(&guard.owner_path, text).map_err(io(&guard.owner_path))?;
    Ok(guard)
}

/// The owner file, or `None` when it is missing or unreadable.
fn read_owner(path: &Path) -> Option<Owner> {
    serde_json::from_str(&std::fs::read_to_string(path).ok()?).ok()
}

/// The file stem of the running executable, else `theos`.
fn program_name() -> String {
    std::env::current_exe()
        .ok()
        .and_then(|exe| Some(exe.file_stem()?.to_string_lossy().into_owned()))
        .unwrap_or_else(|| "theos".to_owned())
}

/// Seconds since the Unix epoch.
fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}
