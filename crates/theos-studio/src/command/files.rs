//! Writes a plan to disk: whole files, each swapped in atomically, put back on a failure.

use std::fs;
use std::io::{self, ErrorKind, Write};
use std::path::{Path, PathBuf};
use std::time::Duration;

use super::{CommandError, FileChange, Plan};

/// How often a refused write is tried again.
pub(crate) const RETRIES: u32 = 5;
/// The pause between two tries.
pub(crate) const RETRY_PAUSE: Duration = Duration::from_millis(100);

/// Writes `plan` under `root`: content first in plan order, removals last, then the folders those
/// removals emptied. A read-only file the plan would replace or remove fails it with `ReadOnly`
/// before anything is written. On a failure it puts back what it wrote from each change's
/// `before`, removes the folders it created that are empty again, and fails with `Write`, or with
/// `Partial` when a file cannot be put back.
pub(crate) fn write_plan(root: &Path, plan: &Plan) -> Result<(), CommandError> {
    write_plan_with(root, plan, &mut DiskOps)
}

/// The two operations a plan is made of. The seam lets the tests fail a chosen call.
pub(crate) trait FileOps {
    /// Writes `bytes` to `target` whole; `replace` says the target exists and is swapped. Each
    /// folder it creates for `target` goes into `created`, the outermost first, also when the
    /// write then fails.
    fn write(
        &mut self,
        target: &Path,
        bytes: &[u8],
        replace: bool,
        created: &mut Vec<PathBuf>,
    ) -> io::Result<()>;
    /// Removes `target`; `false` when there was no file.
    fn remove(&mut self, target: &Path) -> io::Result<bool>;
}

/// The operations on the real disk.
pub(crate) struct DiskOps;

impl FileOps for DiskOps {
    fn write(
        &mut self,
        target: &Path,
        bytes: &[u8],
        replace: bool,
        created: &mut Vec<PathBuf>,
    ) -> io::Result<()> {
        write_file(target, bytes, replace, created)
    }

    fn remove(&mut self, target: &Path) -> io::Result<bool> {
        match with_retries(target, || fs::remove_file(target)) {
            Ok(()) => Ok(true),
            Err(error) if error.kind() == ErrorKind::NotFound => Ok(false),
            Err(error) => Err(error),
        }
    }
}

/// [`write_plan`] over the given operations, for the forward pass and the rollback alike.
pub(crate) fn write_plan_with(
    root: &Path,
    plan: &Plan,
    ops: &mut dyn FileOps,
) -> Result<(), CommandError> {
    let changes: Vec<&FileChange> = plan.real_changes().collect();
    refuse_read_only(root, &changes)?;
    let writes = changes.iter().filter(|change| change.after.is_some());
    let removals = changes.iter().filter(|change| change.after.is_none());
    let mut done = Done::default();
    for change in writes.chain(removals) {
        match apply(root, change, ops, &mut done.folders) {
            Ok(true) => done.changes.push(change),
            // A removal that found no file did nothing; a rollback must not create it.
            Ok(false) => {}
            Err(source) => return Err(roll_back(root, done, change, source, ops)),
        }
    }
    for change in done.changes.iter().filter(|change| change.after.is_none()) {
        remove_emptied_folders(root, &change.path);
    }
    Ok(())
}

/// [`CommandError::ReadOnly`] for the first change whose target exists and has no write
/// permission: the change would replace or remove it. Checked before any change is applied, so
/// nothing needs rolling back, and on every OS alike (ruling R9): Windows would refuse to replace
/// such a file only after every retry, and std removes it on Windows and Unix, as Unix replaces
/// it, without a word.
fn refuse_read_only(root: &Path, changes: &[&FileChange]) -> Result<(), CommandError> {
    for change in changes.iter().filter(|change| change.before.is_some()) {
        let target = root.join(&change.path);
        // The file itself, not what a symbolic link points to: the link is what is replaced.
        if fs::symlink_metadata(&target).is_ok_and(|metadata| metadata.permissions().readonly()) {
            return Err(CommandError::ReadOnly {
                path: change.path.clone(),
            });
        }
    }
    Ok(())
}

/// What the writing of a plan has done so far.
#[derive(Default)]
struct Done<'p> {
    /// The changes applied, in the order they were applied.
    changes: Vec<&'p FileChange>,
    /// The folders created for them, each one after the folders above it.
    folders: Vec<PathBuf>,
}

/// Does one change; `false` when a removal found no file to remove. The folders it creates go into
/// `created`.
fn apply(
    root: &Path,
    change: &FileChange,
    ops: &mut dyn FileOps,
    created: &mut Vec<PathBuf>,
) -> io::Result<bool> {
    let target = root.join(&change.path);
    if let Some(after) = &change.after {
        ops.write(&target, after, change.before.is_some(), created)?;
        return Ok(true);
    }
    ops.remove(&target)
}

/// Puts back what `done` wrote, removes the folders it created that are empty again, then builds
/// the error for the failed `change`. A folder that was there before the plan stays, empty or not.
fn roll_back(
    root: &Path,
    mut done: Done<'_>,
    failed: &FileChange,
    source: io::Error,
    ops: &mut dyn FileOps,
) -> CommandError {
    let mut unrestored = Vec::new();
    for change in done.changes.iter().rev() {
        let target = root.join(&change.path);
        tracing::warn!(path = %change.path, "rolling back");
        let restored = match &change.before {
            Some(before) => ops.write(&target, before, change.after.is_some(), &mut done.folders),
            None => ops.remove(&target).map(drop),
        };
        if let Err(error) = restored {
            tracing::warn!(path = %change.path, %error, "cannot put the file back");
            unrestored.push(change.path.clone());
        }
    }
    remove_created_folders(&done.folders);
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
/// atomically, a new file never replaces one that appeared meanwhile. The folders it creates for
/// `target` go into `created`.
fn write_file(
    target: &Path,
    bytes: &[u8],
    replace: bool,
    created: &mut Vec<PathBuf>,
) -> io::Result<()> {
    let parent = target.parent().unwrap_or_else(|| Path::new("."));
    create_folders(parent, created)?;
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
                Err(error) if retryable(&error.error) => return Err(exhausted(error.error)),
                Err(error) => return Err(error.error),
            }
        }
    }
}

/// Creates `folder` with every missing folder above it, as [`fs::create_dir_all`] does, and adds
/// each folder it creates to `created`, the outermost first. `create_dir_all` does not say which
/// folders it made, and a rollback removes exactly those: so this walks up to the first folder
/// that exists, then creates the missing ones one by one. A folder that another program creates
/// meanwhile is not counted.
fn create_folders(folder: &Path, created: &mut Vec<PathBuf>) -> io::Result<()> {
    let mut missing = Vec::new();
    let mut next = Some(folder);
    while let Some(folder) = next {
        // A folder that cannot be looked at counts as there: creating it would fail anyway.
        if folder.as_os_str().is_empty() || !matches!(folder.try_exists(), Ok(false)) {
            break;
        }
        missing.push(folder);
        next = folder.parent();
    }
    for folder in missing.into_iter().rev() {
        match fs::create_dir(folder) {
            Ok(()) => created.push(folder.to_path_buf()),
            Err(error) if error.kind() == ErrorKind::AlreadyExists && folder.is_dir() => {}
            Err(error) => return Err(error),
        }
    }
    Ok(())
}

/// Removes each folder of `created` that is empty, the deepest first: `created` lists each folder
/// after the folders above it. A folder that holds anything stays.
fn remove_created_folders(created: &[PathBuf]) {
    for folder in created.iter().rev() {
        // `remove_dir` removes an empty folder only.
        match fs::remove_dir(folder) {
            Ok(()) => {}
            Err(error)
                if matches!(
                    error.kind(),
                    ErrorKind::DirectoryNotEmpty | ErrorKind::NotFound
                ) => {}
            Err(error) => {
                tracing::warn!(folder = %folder.display(), %error, "cannot remove a folder the command created");
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
            Err(error) if retryable(&error) => return Err(exhausted(error)),
            other => return other,
        }
    }
}

/// Marks an error that stayed after every retry; it shows the same message and kind.
#[derive(Debug)]
struct Exhausted(io::Error);

impl std::fmt::Display for Exhausted {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl std::error::Error for Exhausted {}

/// `error` marked as out of retries.
fn exhausted(error: io::Error) -> io::Error {
    io::Error::new(error.kind(), Exhausted(error))
}

/// Whether `error` came out of a retry loop that used up its tries.
pub(super) fn was_retried(error: &io::Error) -> bool {
    error
        .get_ref()
        .is_some_and(<dyn std::error::Error + Send + Sync>::is::<Exhausted>)
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
    use crate::command::failing_ops::{Fail, FailingOps, Op};
    use crate::test_support::{ReadOnly, TestResult, change, entries, plan};

    /// The temporary files of the writer left under `dir`, joined.
    fn temp_files(dir: &Path) -> String {
        let temp: Vec<String> = entries(dir)
            .into_iter()
            .filter(|(path, is_folder)| {
                !is_folder
                    && path
                        .file_name()
                        .is_some_and(|name| name.to_string_lossy().starts_with(".theos-"))
            })
            .map(|(path, _)| path.display().to_string())
            .collect();
        temp.join(", ")
    }

    #[test]
    fn an_error_that_was_not_retried_has_no_hint() {
        let error = CommandError::Write {
            path: "a.md".into(),
            source: io::Error::from(ErrorKind::PermissionDenied),
        };
        assert!(!error.to_string().contains("another program"), "{error}");
    }

    #[test]
    fn an_error_out_of_the_retries_has_the_hint() {
        let mut tries = 0;
        let source = with_retries(Path::new("a.md"), || {
            tries += 1;
            Err(io::Error::from(ErrorKind::PermissionDenied))
        })
        .expect_err("the operation always fails");
        assert_eq!(tries, RETRIES + 1);
        assert_eq!(source.kind(), ErrorKind::PermissionDenied);
        let error = CommandError::Write {
            path: "a.md".into(),
            source,
        };
        assert!(
            error
                .to_string()
                .contains("another program may have it open"),
            "{error}"
        );
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
        // The create was never retried: the message does not blame another program.
        let message = result
            .err()
            .map(|error| error.to_string())
            .unwrap_or_default();
        assert!(!message.contains("another program"), "{message}");
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
        let moved = plan(vec![
            change("old/inner/x.md", Some("x"), None),
            change("new/x.md", None, Some("x")),
        ]);
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
        // Windows retries the refused swap, so its error carries the hint; on Unix the folder
        // refuses the temp file at once, so nothing was retried and there is no hint.
        assert_eq!(
            message.contains("another program may have it open"),
            cfg!(windows),
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

    /// Ruling R9, on every OS: a read-only file that a plan would replace or remove is refused
    /// before anything is written.
    #[test]
    fn a_read_only_target_is_refused_before_anything_is_written() -> TestResult {
        for after in [Some("ro-new"), None] {
            let dir = tempfile::tempdir()?;
            fs::write(dir.path().join("first.md"), "first-old")?;
            fs::write(dir.path().join("ro.md"), "ro-old")?;
            let _read_only = ReadOnly::set(dir.path().join("ro.md"))?;
            let result = write_plan(
                dir.path(),
                &plan(vec![
                    change("first.md", Some("first-old"), Some("first-new")),
                    change("fresh/new.md", None, Some("new")),
                    change("ro.md", Some("ro-old"), after),
                ]),
            );
            assert!(
                matches!(&result, Err(CommandError::ReadOnly { path }) if path == "ro.md"),
                "{after:?}: {result:?}"
            );
            assert_eq!(
                result.err().map(|error| error.to_string()),
                Some("`ro.md` is read-only; make it writable first".to_owned())
            );
            assert_eq!(fs::read_to_string(dir.path().join("ro.md"))?, "ro-old");
            assert_eq!(
                fs::read_to_string(dir.path().join("first.md"))?,
                "first-old"
            );
            assert!(!dir.path().join("fresh").exists());
            assert_eq!(temp_files(dir.path()), "");
        }
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
        fs::create_dir_all(dir.path().join("a"))?;
        fs::create_dir_all(dir.path().join("b"))?;
        fs::write(dir.path().join("a/first.md"), "first-old")?;
        fs::write(dir.path().join("b/second.md"), "second-old")?;
        let mut ops = FailingOps::new(vec![
            Fail::new(Op::Write, "b/second.md", 1),
            Fail::new(Op::Write, "a/first.md", 2),
        ]);
        let result = write_plan_with(
            dir.path(),
            &plan(vec![
                change("a/first.md", Some("first-old"), Some("first-new")),
                change("b/second.md", Some("second-old"), Some("second-new")),
            ]),
            &mut ops,
        );
        match result {
            Err(CommandError::Partial {
                path, unrestored, ..
            }) => {
                assert_eq!(path, "b/second.md");
                assert_eq!(unrestored, ["a/first.md"]);
            }
            other => return Err(format!("expected Partial, got {other:?}").into()),
        }
        assert_eq!(
            fs::read_to_string(dir.path().join("a/first.md"))?,
            "first-new"
        );
        Ok(())
    }

    #[test]
    fn a_removed_file_is_restored_from_its_before_bytes() -> TestResult {
        let dir = tempfile::tempdir()?;
        let bytes: &[u8] = b"one\r\ntwo\n\xff";
        fs::create_dir_all(dir.path().join("r"))?;
        fs::write(dir.path().join("r/one.md"), bytes)?;
        fs::write(dir.path().join("r/two.md"), "two")?;
        let removal = |path: &str, before: &[u8]| FileChange {
            path: path.to_owned(),
            before: Some(before.to_vec()),
            after: None,
        };
        let mut ops = FailingOps::new(vec![Fail::new(Op::Remove, "r/two.md", 1)]);
        let result = write_plan_with(
            dir.path(),
            &plan(vec![
                removal("r/one.md", bytes),
                removal("r/two.md", b"two"),
            ]),
            &mut ops,
        );
        assert!(matches!(result, Err(CommandError::Write { ref path, .. }) if path == "r/two.md"));
        assert_eq!(fs::read(dir.path().join("r/one.md"))?, bytes);
        assert_eq!(fs::read_to_string(dir.path().join("r/two.md"))?, "two");
        assert_eq!(temp_files(dir.path()), "");
        Ok(())
    }

    /// Every folder under `dir`, as paths relative to it with `/`.
    fn folders(dir: &Path) -> Vec<String> {
        let mut found: Vec<String> = entries(dir)
            .into_iter()
            .filter(|(_, is_folder)| *is_folder)
            .map(|(path, _)| {
                let relative = path.strip_prefix(dir).unwrap_or(&path);
                relative.to_string_lossy().replace('\\', "/")
            })
            .collect();
        found.sort();
        found
    }

    #[test]
    fn a_rollback_removes_the_folders_the_plan_created() -> TestResult {
        for failing in [
            Fail::new(Op::Write, "c/third.md", 1),
            Fail::new(Op::Remove, "old.md", 1),
        ] {
            let dir = tempfile::tempdir()?;
            fs::create_dir_all(dir.path().join("kept/empty"))?;
            fs::create_dir_all(dir.path().join("c"))?;
            fs::write(dir.path().join("old.md"), "old")?;
            fs::write(dir.path().join("c/third.md"), "third-old")?;
            let mut ops = FailingOps::new(vec![failing]);
            let result = write_plan_with(
                dir.path(),
                &plan(vec![
                    change("old.md", Some("old"), None),
                    change("kept/new/deep/first.md", None, Some("first")),
                    change("fresh/second.md", None, Some("second")),
                    change("c/third.md", Some("third-old"), Some("third-new")),
                ]),
                &mut ops,
            );
            assert!(
                matches!(result, Err(CommandError::Write { .. })),
                "{result:?}"
            );
            // The folders the plan created are gone; the empty folder that was there stays.
            assert_eq!(folders(dir.path()), ["c", "kept", "kept/empty"]);
            assert_eq!(fs::read_to_string(dir.path().join("old.md"))?, "old");
            assert_eq!(
                fs::read_to_string(dir.path().join("c/third.md"))?,
                "third-old"
            );
        }
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
