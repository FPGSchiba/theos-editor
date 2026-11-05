//! Which files of a content project are records.
//!
//! The index walks the project with `ignore`: hidden folders are skipped, `.gitignore` files
//! inside the project are honoured (never those of parent folders or the user's global one), and
//! P01's not-records (`README.md`, `templates/records/`, `tools/`, `attachments/`) are left out.

use std::path::{Component, Path};
use std::time::UNIX_EPOCH;

use ignore::WalkBuilder;

/// A project-relative record path, with `/` on every OS.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WalkedPath {
    /// A path that is valid UTF-8.
    Utf8(String),
    /// A path with a non-UTF-8 component, in its lossy form.
    NotUtf8(String),
}

impl WalkedPath {
    fn as_str(&self) -> &str {
        match self {
            Self::Utf8(s) | Self::NotUtf8(s) => s,
        }
    }
}

/// A record file found by [`walk`].
#[derive(Clone, Debug)]
pub struct Walked {
    /// The project-relative path.
    pub path: WalkedPath,
    /// The size of the file in bytes.
    pub size: u64,
    /// The modification time in nanoseconds since the UNIX epoch (negative before it); 0 when the
    /// file system gives no time, so that the sync treats the file as racy and reads it.
    pub mtime_ns: i64,
}

/// The walk of a project failed; the whole walk fails, since a partial one would report records
/// it did not see as removed.
#[derive(Debug, thiserror::Error)]
#[error("cannot walk the project: {source}")]
pub struct WalkError {
    #[from]
    source: ignore::Error,
}

/// Whether a project-relative path (with `/`) is a record: it ends in `.md`, is not a
/// `README.md`, and is not under `templates/records/`, `tools/` or `attachments/`.
#[must_use]
pub fn is_record_path(path: &str) -> bool {
    let name = path.rsplit('/').next().unwrap_or(path);
    path.strip_suffix(".md").is_some()
        && name != "README.md"
        && !["templates/records/", "tools/", "attachments/"]
            .iter()
            .any(|prefix| path.starts_with(prefix))
}

/// The modification time of a file in nanoseconds since the UNIX epoch, negative before it.
/// Returns 0 when the platform cannot tell; the sync then reads the file as racy.
#[must_use]
pub fn mtime_ns(meta: &std::fs::Metadata) -> i64 {
    let Ok(time) = meta.modified() else {
        return 0;
    };
    match time.duration_since(UNIX_EPOCH) {
        Ok(after) => i64::try_from(after.as_nanos()).unwrap_or(i64::MAX),
        Err(before) => i64::try_from(before.duration().as_nanos()).map_or(i64::MIN, |n| -n),
    }
}

/// The project-relative form of `path`, or `None` when it is not a record.
pub(crate) fn relative(root: &Path, path: &Path) -> Option<WalkedPath> {
    let rel = path.strip_prefix(root).ok()?;
    let mut parts = Vec::new();
    let mut lossy = false;
    for component in rel.components() {
        let Component::Normal(name) = component else {
            return None;
        };
        lossy |= name.to_str().is_none();
        parts.push(name.to_string_lossy().into_owned());
    }
    let joined = parts.join("/");
    if !is_record_path(&joined) {
        return None;
    }
    Some(if lossy {
        WalkedPath::NotUtf8(joined)
    } else {
        WalkedPath::Utf8(joined)
    })
}

/// Walks `root` and returns its record files sorted by path.
///
/// # Errors
///
/// Returns [`WalkError`] when any entry cannot be read.
pub fn walk(root: &Path) -> Result<Vec<Walked>, WalkError> {
    let walker = WalkBuilder::new(root)
        .hidden(true)
        .parents(false)
        .ignore(false)
        .git_ignore(true)
        .git_global(false)
        .git_exclude(false)
        .require_git(false)
        .follow_links(false)
        .build();
    let mut found = Vec::new();
    for entry in walker {
        let entry = entry?;
        if !entry.file_type().is_some_and(|t| t.is_file()) {
            continue;
        }
        let Some(path) = relative(root, entry.path()) else {
            continue;
        };
        let meta = entry.metadata()?;
        found.push(Walked {
            path,
            size: meta.len(),
            mtime_ns: mtime_ns(&meta),
        });
    }
    found.sort_by(|a, b| a.path.as_str().cmp(b.path.as_str()));
    Ok(found)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::Path;
    use std::time::{Duration, UNIX_EPOCH};

    fn put(root: &Path, rel: &str, text: &str) {
        let path = root.join(rel);
        fs::create_dir_all(path.parent().expect("has a parent")).expect("mkdir");
        fs::write(path, text).expect("write");
    }

    #[test]
    fn records_and_not_records() {
        let dir = tempfile::tempdir().expect("tempdir");
        let root = dir.path();
        for rel in [
            "a.md",
            "world/tower.md",
            "templates/synthetic-template.md",
            "README.md",
            "world/README.md",
            "templates/records/npc.template.md",
            "tools/x.md",
            "attachments/x.md",
            "notes.MD",
            "x.txt",
            ".obsidian/x.md",
            ".theos/cache/x.md",
            "lore/.hidden.md",
            "drafts/a.md",
            "sub/b.md",
        ] {
            put(root, rel, "x\n");
        }
        put(root, ".gitignore", "drafts/\n");
        put(root, "sub/.gitignore", "b.md\n");
        let walked = walk(root).expect("walk");
        let paths: Vec<_> = walked.iter().map(|w| w.path.clone()).collect();
        let want: Vec<_> = ["a.md", "templates/synthetic-template.md", "world/tower.md"]
            .iter()
            .map(|p| WalkedPath::Utf8((*p).to_owned()))
            .collect();
        assert_eq!(paths, want);
    }

    #[test]
    fn is_record_path_follows_p01() {
        let table = [
            ("a.md", true),
            ("x.md", true),
            ("world/tower.md", true),
            ("templates/synthetic-template.md", true),
            ("README.md", false),
            ("world/README.md", false),
            ("templates/records/npc.template.md", false),
            ("tools/x.md", false),
            ("tools/a/b.md", false),
            ("attachments/x.md", false),
            ("notes.MD", false),
            ("x.txt", false),
        ];
        for (path, want) in table {
            assert_eq!(is_record_path(path), want, "{path}");
        }
    }

    #[test]
    fn sizes_and_times_come_from_the_walk() {
        let dir = tempfile::tempdir().expect("tempdir");
        put(dir.path(), "a.md", "hello\n");
        let file = fs::File::options()
            .write(true)
            .open(dir.path().join("a.md"))
            .expect("open");
        file.set_modified(UNIX_EPOCH + Duration::new(1_700_000_000, 123_456_700))
            .expect("set time");
        drop(file);
        let walked = walk(dir.path()).expect("walk");
        assert_eq!(walked.len(), 1);
        assert_eq!(walked[0].size, 6);
        assert_eq!(walked[0].mtime_ns, 1_700_000_000_123_456_700);
    }

    #[test]
    fn a_missing_root_fails_the_walk() {
        let dir = tempfile::tempdir().expect("tempdir");
        let err = walk(&dir.path().join("nope")).expect_err("must fail");
        assert!(err.to_string().starts_with("cannot walk the project"));
    }

    #[test]
    fn times_before_the_epoch_are_negative() {
        let dir = tempfile::tempdir().expect("tempdir");
        put(dir.path(), "a.md", "x");
        let file = fs::File::options()
            .write(true)
            .open(dir.path().join("a.md"))
            .expect("open");
        if file
            .set_modified(UNIX_EPOCH - Duration::new(5, 500_000_000))
            .is_err()
        {
            return;
        }
        drop(file);
        let meta = fs::metadata(dir.path().join("a.md")).expect("meta");
        assert_eq!(mtime_ns(&meta), -5_500_000_000);
    }

    #[cfg(unix)]
    #[test]
    fn symbolic_links_are_not_records() {
        let dir = tempfile::tempdir().expect("tempdir");
        put(dir.path(), "a.md", "x");
        std::os::unix::fs::symlink(dir.path().join("a.md"), dir.path().join("l.md"))
            .expect("symlink");
        let walked = walk(dir.path()).expect("walk");
        let paths: Vec<_> = walked.iter().map(|w| w.path.clone()).collect();
        assert_eq!(paths, vec![WalkedPath::Utf8("a.md".to_owned())]);
    }

    #[cfg(unix)]
    #[test]
    fn a_non_utf8_name_is_kept_lossy() {
        use std::os::unix::ffi::OsStrExt;
        let root = Path::new("/p");
        let path = root.join(std::ffi::OsStr::from_bytes(b"x\xff.md"));
        match relative(root, &path) {
            Some(WalkedPath::NotUtf8(s)) => assert!(s.strip_suffix(".md").is_some()),
            other => panic!("unexpected {other:?}"),
        }
    }

    #[cfg(windows)]
    #[test]
    fn a_non_utf8_name_is_kept_lossy_on_windows() {
        use std::os::windows::ffi::OsStringExt;
        let root = Path::new("C:\\p");
        let name = std::ffi::OsString::from_wide(&[0x78, 0xD800, 0x2E, 0x6D, 0x64]);
        let path = root.join(name);
        match relative(root, &path) {
            Some(WalkedPath::NotUtf8(s)) => assert!(s.strip_suffix(".md").is_some()),
            other => panic!("unexpected {other:?}"),
        }
    }
}
