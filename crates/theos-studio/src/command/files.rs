//! Writes a plan to disk: whole files, each swapped in atomically, put back on a failure.

#![cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "the pipeline of P06 calls the writer; remove this then"
    )
)]

use std::fs;
use std::io::{self, ErrorKind, Write};
use std::path::Path;
use std::time::Duration;

use super::{CommandError, FileChange, Plan};

/// How often a refused write is tried again.
pub(crate) const RETRIES: u32 = 5;
/// The pause between two tries.
pub(crate) const RETRY_PAUSE: Duration = Duration::from_millis(100);

/// Writes `plan` under `root`: content first in plan order, removals last, then the folders those
/// removals emptied. On a failure it puts back what it wrote from each change's `before` and fails
/// with `Write`, or with `Partial` when that fails too.
pub(crate) fn write_plan(root: &Path, plan: &Plan) -> Result<(), CommandError> {
    let changes: Vec<&FileChange> = plan
        .changes
        .iter()
        .filter(|change| change.before != change.after)
        .collect();
    let writes = changes.iter().filter(|change| change.after.is_some());
    let removals = changes.iter().filter(|change| change.after.is_none());
    let mut done: Vec<&FileChange> = Vec::new();
    for change in writes.chain(removals) {
        match apply(root, change) {
            Ok(true) => done.push(change),
            // A removal that found no file did nothing; a rollback must not create it.
            Ok(false) => {}
            Err(source) => return Err(roll_back(root, &done, change, source)),
        }
    }
    for change in done.iter().filter(|change| change.after.is_none()) {
        remove_emptied_folders(root, &change.path);
    }
    Ok(())
}

/// Does one change; `false` when a removal found no file to remove.
fn apply(root: &Path, change: &FileChange) -> io::Result<bool> {
    let target = root.join(&change.path);
    if let Some(after) = &change.after {
        write_file(&target, after, change.before.is_some())?;
        return Ok(true);
    }
    match with_retries(&target, || fs::remove_file(&target)) {
        Ok(()) => Ok(true),
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(false),
        Err(error) => Err(error),
    }
}

/// Puts back what `done` wrote, then builds the error for the failed `change`.
fn roll_back(
    root: &Path,
    done: &[&FileChange],
    failed: &FileChange,
    source: io::Error,
) -> CommandError {
    let mut unrestored = Vec::new();
    for change in done.iter().rev() {
        let target = root.join(&change.path);
        tracing::warn!(path = %change.path, "rolling back");
        let restored = match &change.before {
            Some(before) => write_file(&target, before, change.after.is_some()),
            None => with_retries(&target, || fs::remove_file(&target)),
        };
        if let Err(error) = restored {
            tracing::warn!(path = %change.path, %error, "cannot put the file back");
            unrestored.push(change.path.clone());
        }
    }
    let path = failed.path.clone();
    if unrestored.is_empty() {
        CommandError::Write { path, source }
    } else {
        unrestored.reverse();
        CommandError::Partial {
            path,
            source,
            unrestored,
        }
    }
}

/// Writes `bytes` to `target` whole: a replacement keeps the target's permissions and swaps in
/// atomically, a new file never replaces one that appeared meanwhile.
fn write_file(target: &Path, bytes: &[u8], replace: bool) -> io::Result<()> {
    let parent = target.parent().unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(parent)?;
    let mut builder = tempfile::Builder::new();
    builder.prefix(".theos-").suffix(".tmp");
    #[cfg(unix)]
    if !replace {
        use std::os::unix::fs::PermissionsExt;
        builder.permissions(fs::Permissions::from_mode(0o666));
    }
    let mut temp = builder.tempfile_in(parent)?;
    temp.write_all(bytes)?;
    temp.as_file().sync_all()?;
    if replace {
        if let Ok(metadata) = fs::metadata(target) {
            temp.as_file().set_permissions(metadata.permissions())?;
        }
        let kept = temp.into_temp_path().keep().map_err(|error| error.error)?;
        let result = with_retries(target, || fs::rename(&kept, target));
        if result.is_err() {
            let _ = fs::remove_file(&kept);
        }
        result
    } else {
        let mut attempt = 0;
        loop {
            match temp.persist_noclobber(target) {
                Ok(_) => return Ok(()),
                Err(error) if retryable(&error.error) && attempt < RETRIES => {
                    attempt += 1;
                    tracing::warn!(path = %target.display(), error = %error.error, attempt, "write refused, trying again");
                    std::thread::sleep(RETRY_PAUSE);
                    temp = error.file;
                }
                Err(error) => return Err(error.error),
            }
        }
    }
}

/// Runs `op`, again after a pause while the system refuses it for a reason that passes.
fn with_retries(target: &Path, mut op: impl FnMut() -> io::Result<()>) -> io::Result<()> {
    let mut attempt = 0;
    loop {
        match op() {
            Err(error) if retryable(&error) && attempt < RETRIES => {
                attempt += 1;
                tracing::warn!(path = %target.display(), %error, attempt, "write refused, trying again");
                std::thread::sleep(RETRY_PAUSE);
            }
            other => return other,
        }
    }
}

/// Whether another program may be holding the file only for now.
fn retryable(error: &io::Error) -> bool {
    if error.kind() == ErrorKind::PermissionDenied {
        return true;
    }
    // ERROR_SHARING_VIOLATION
    cfg!(windows) && error.raw_os_error() == Some(32)
}

/// Removes the folders above `path`, from the inside out, while they are empty.
fn remove_emptied_folders(root: &Path, path: &str) {
    let mut folder = root.join(path);
    while folder.pop() && folder != root && folder.starts_with(root) {
        if fs::remove_dir(&folder).is_err() {
            break;
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;
    use crate::command::FileChange;
    use crate::command::test_support::TestResult;

    fn change(path: &str, before: Option<&str>, after: Option<&str>) -> FileChange {
        FileChange {
            path: path.to_owned(),
            before: before.map(|s| s.as_bytes().to_vec()),
            after: after.map(|s| s.as_bytes().to_vec()),
        }
    }

    fn plan(changes: Vec<FileChange>) -> Plan {
        Plan {
            changes,
            ..Plan::default()
        }
    }

    fn temp_files(dir: &Path) -> String {
        let mut found = Vec::new();
        let mut stack = vec![dir.to_path_buf()];
        while let Some(folder) = stack.pop() {
            for entry in fs::read_dir(&folder).into_iter().flatten().flatten() {
                let path = entry.path();
                if path.is_dir() {
                    stack.push(path);
                } else if entry.file_name().to_string_lossy().starts_with(".theos-") {
                    found.push(path.display().to_string());
                }
            }
        }
        found.join(", ")
    }

    #[test]
    fn writes_new_files_and_replaces_existing_ones() -> TestResult {
        let dir = tempfile::tempdir()?;
        fs::write(dir.path().join("old.md"), "one")?;
        write_plan(
            dir.path(),
            &plan(vec![
                change("old.md", Some("one"), Some("two")),
                change("sub/deep/new.md", None, Some("fresh")),
            ]),
        )?;
        assert_eq!(fs::read_to_string(dir.path().join("old.md"))?, "two");
        assert_eq!(
            fs::read_to_string(dir.path().join("sub/deep/new.md"))?,
            "fresh"
        );
        assert_eq!(temp_files(dir.path()), "");
        Ok(())
    }

    #[test]
    fn a_create_never_replaces_a_file_that_appeared() -> TestResult {
        let dir = tempfile::tempdir()?;
        fs::write(dir.path().join("a.md"), "a-old")?;
        fs::write(dir.path().join("b.md"), "appeared")?;
        let result = write_plan(
            dir.path(),
            &plan(vec![
                change("a.md", Some("a-old"), Some("a-new")),
                change("b.md", None, Some("mine")),
            ]),
        );
        assert!(matches!(result, Err(CommandError::Write { ref path, .. }) if path == "b.md"));
        assert_eq!(fs::read_to_string(dir.path().join("b.md"))?, "appeared");
        assert_eq!(fs::read_to_string(dir.path().join("a.md"))?, "a-old");
        assert_eq!(temp_files(dir.path()), "");
        Ok(())
    }

    #[test]
    fn removals_come_last_and_emptied_folders_go() -> TestResult {
        let dir = tempfile::tempdir()?;
        fs::create_dir_all(dir.path().join("old/inner"))?;
        fs::create_dir_all(dir.path().join("unrelated"))?;
        fs::write(dir.path().join("old/inner/x.md"), "x")?;
        let moved = Plan {
            changes: vec![
                change("old/inner/x.md", Some("x"), None),
                change("new/x.md", None, Some("x")),
            ],
            ..Plan::default()
        };
        write_plan(dir.path(), &moved)?;
        assert_eq!(fs::read_to_string(dir.path().join("new/x.md"))?, "x");
        assert!(!dir.path().join("old").exists());
        assert!(dir.path().join("unrelated").is_dir());
        Ok(())
    }

    #[test]
    fn a_failure_rolls_back_every_written_file() -> TestResult {
        let dir = tempfile::tempdir()?;
        fs::create_dir_all(dir.path().join("locked"))?;
        fs::write(dir.path().join("first.md"), "first-old")?;
        fs::write(dir.path().join("locked/second.md"), "second-old")?;
        let changes = plan(vec![
            change("first.md", Some("first-old"), Some("first-new")),
            change("locked/second.md", Some("second-old"), Some("second-new")),
        ]);

        #[cfg(windows)]
        let (result, _guard) = {
            use std::os::windows::fs::OpenOptionsExt;
            let held = fs::OpenOptions::new()
                .read(true)
                .share_mode(1 | 2)
                .open(dir.path().join("locked/second.md"))?;
            (write_plan(dir.path(), &changes), held)
        };
        #[cfg(unix)]
        let result = {
            use std::os::unix::fs::PermissionsExt;
            let locked = dir.path().join("locked");
            fs::set_permissions(&locked, fs::Permissions::from_mode(0o555))?;
            let result = write_plan(dir.path(), &changes);
            fs::set_permissions(&locked, fs::Permissions::from_mode(0o755))?;
            result
        };

        let Err(error) = result else {
            return Err("the write should have failed".into());
        };
        let message = error.to_string();
        assert!(message.contains("locked/second.md"), "{message}");
        assert!(
            message.contains("another program may have it open"),
            "{message}"
        );
        assert!(matches!(error, CommandError::Write { .. }), "{error:?}");
        assert_eq!(
            fs::read_to_string(dir.path().join("first.md"))?,
            "first-old"
        );
        assert_eq!(
            fs::read_to_string(dir.path().join("locked/second.md"))?,
            "second-old"
        );
        assert_eq!(temp_files(dir.path()), "");
        Ok(())
    }

    /// Holds `dir/file` so that it cannot be replaced or removed: an open handle without delete
    /// sharing on Windows, a read-only folder on Unix.
    #[cfg(windows)]
    fn lock_down(dir: &Path, file: &str) -> io::Result<fs::File> {
        use std::os::windows::fs::OpenOptionsExt;
        fs::OpenOptions::new()
            .read(true)
            .share_mode(1 | 2)
            .open(dir.join(file))
    }

    #[cfg(unix)]
    struct Held(std::path::PathBuf);

    #[cfg(unix)]
    impl Drop for Held {
        fn drop(&mut self) {
            use std::os::unix::fs::PermissionsExt;
            let _ = fs::set_permissions(&self.0, fs::Permissions::from_mode(0o755));
        }
    }

    #[cfg(unix)]
    fn lock_down(dir: &Path, _file: &str) -> io::Result<Held> {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(dir, fs::Permissions::from_mode(0o555))?;
        Ok(Held(dir.to_path_buf()))
    }

    #[test]
    fn a_failed_removal_rolls_back_the_writes_and_keeps_the_file() -> TestResult {
        let dir = tempfile::tempdir()?;
        fs::create_dir_all(dir.path().join("b"))?;
        fs::write(dir.path().join("first.md"), "first-old")?;
        fs::write(dir.path().join("b/second.md"), "second-old")?;
        let held = lock_down(&dir.path().join("b"), "second.md")?;
        let result = write_plan(
            dir.path(),
            &plan(vec![
                change("first.md", Some("first-old"), Some("first-new")),
                change("b/second.md", Some("second-old"), None),
            ]),
        );
        drop(held);
        assert!(
            matches!(result, Err(CommandError::Write { ref path, .. }) if path == "b/second.md")
        );
        assert_eq!(
            fs::read_to_string(dir.path().join("first.md"))?,
            "first-old"
        );
        assert_eq!(
            fs::read_to_string(dir.path().join("b/second.md"))?,
            "second-old"
        );
        assert_eq!(temp_files(dir.path()), "");
        Ok(())
    }

    #[test]
    fn a_removal_of_a_missing_file_is_not_rolled_back_into_existence() -> TestResult {
        let dir = tempfile::tempdir()?;
        fs::create_dir_all(dir.path().join("b"))?;
        fs::write(dir.path().join("b/second.md"), "second-old")?;
        let held = lock_down(&dir.path().join("b"), "second.md")?;
        let result = write_plan(
            dir.path(),
            &plan(vec![
                change("gone.md", Some("was here"), None),
                change("b/second.md", Some("second-old"), None),
            ]),
        );
        drop(held);
        assert!(
            matches!(result, Err(CommandError::Write { ref path, .. }) if path == "b/second.md")
        );
        assert!(!dir.path().join("gone.md").exists());
        Ok(())
    }

    #[test]
    fn a_restore_that_fails_too_is_reported_as_partial() -> TestResult {
        let dir = tempfile::tempdir()?;
        let (a, b) = (dir.path().join("a"), dir.path().join("b"));
        fs::create_dir_all(&a)?;
        fs::create_dir_all(&b)?;
        fs::write(a.join("first.md"), "first-old")?;
        fs::write(b.join("second.md"), "second-old")?;
        let held_second = lock_down(&b, "second.md")?;
        // Once the first file has its new bytes, hold it too, so that putting it back fails.
        // The refused second write is tried again for about half a second, which is the window.
        let watcher = {
            let a = a.clone();
            std::thread::spawn(move || {
                for _ in 0..1000 {
                    if fs::read(a.join("first.md")).is_ok_and(|bytes| bytes == b"first-new") {
                        return lock_down(&a, "first.md").map_err(|error| error.to_string());
                    }
                    std::thread::sleep(Duration::from_millis(1));
                }
                Err("the first file never got its new bytes".to_owned())
            })
        };
        let result = write_plan(
            dir.path(),
            &plan(vec![
                change("a/first.md", Some("first-old"), Some("first-new")),
                change("b/second.md", Some("second-old"), Some("second-new")),
            ]),
        );
        let held_first = watcher.join().map_err(|_| "the watcher panicked")?;
        drop(held_first);
        drop(held_second);
        match result {
            Err(CommandError::Partial {
                path, unrestored, ..
            }) => {
                assert_eq!(path, "b/second.md");
                assert_eq!(unrestored, ["a/first.md"]);
            }
            other => return Err(format!("expected Partial, got {other:?}").into()),
        }
        assert_eq!(fs::read_to_string(b.join("second.md"))?, "second-old");
        assert_eq!(temp_files(dir.path()), "");
        Ok(())
    }

    #[cfg(unix)]
    #[test]
    fn replaced_files_keep_their_permissions() -> TestResult {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir()?;
        let path = dir.path().join("a.md");
        fs::write(&path, "one")?;
        fs::set_permissions(&path, fs::Permissions::from_mode(0o640))?;
        write_plan(
            dir.path(),
            &plan(vec![change("a.md", Some("one"), Some("two"))]),
        )?;
        assert_eq!(fs::metadata(&path)?.permissions().mode() & 0o777, 0o640);
        Ok(())
    }

    #[cfg(unix)]
    #[test]
    fn new_files_get_the_default_mode() -> TestResult {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir()?;
        let reference = dir.path().join("reference");
        fs::File::create(&reference)?;
        write_plan(dir.path(), &plan(vec![change("new.md", None, Some("x"))]))?;
        let mode = |name: &str| -> Result<u32, std::io::Error> {
            Ok(fs::metadata(dir.path().join(name))?.permissions().mode() & 0o777)
        };
        assert_eq!(mode("new.md")?, mode("reference")?);
        Ok(())
    }
}
