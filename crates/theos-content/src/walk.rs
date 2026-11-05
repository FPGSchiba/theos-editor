//! Which files of a content project are records.
//!
//! The index walks the project with `ignore`: hidden folders are skipped, `.gitignore` files
//! inside the project are honoured (never those of parent folders or the user's global one), and
//! P01's not-records (`README.md`, `templates/records/`, `tools/`, `attachments/`) are left out.

use std::path::{Component, Path, PathBuf};
use std::time::UNIX_EPOCH;

use ignore::WalkBuilder;
use ignore::gitignore::{Gitignore, GitignoreBuilder};

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

/// Whether a project-relative path (with `/`) is a record: a plain relative path (not empty, not
/// starting with `/`, no `\` or `:`, no empty, `.` or `..` component, no component starting
/// with `.`, which the walk skips as hidden) that ends in `.md`, is not a `README.md`, and is
/// not under `templates/records/`, `tools/` or `attachments/`.
#[must_use]
pub fn is_record_path(path: &str) -> bool {
    let plain = !path.contains(['\\', ':'])
        && path
            .split('/')
            .all(|part| !part.is_empty() && !part.starts_with('.'));
    let name = path.rsplit('/').next().unwrap_or(path);
    plain
        && path.strip_suffix(".md").is_some()
        && name != "README.md"
        && !["templates/records/", "tools/", "attachments/"]
            .iter()
            .any(|prefix| path.starts_with(prefix))
}

/// Whether the `.gitignore` files inside the project at `root` hide the record path `path`
/// (project-relative, with `/`) from [`walk`]. As in the walk, each folder of the path and then
/// the file are matched against the `.gitignore` of every folder above them, the deepest first,
/// and the first file that has a rule for them decides; a hidden folder hides everything below
/// it, also a file that a `!` line names. Lines that do not parse are skipped, as the walk skips
/// them.
pub(crate) fn is_ignored(root: &Path, path: &str) -> bool {
    let parts: Vec<&str> = path.split('/').collect();
    let mut matchers: Vec<Gitignore> = Vec::new();
    let mut entry = root.to_path_buf();
    for (depth, part) in parts.iter().enumerate() {
        matchers.push(gitignore(&entry));
        entry.push(part);
        let is_dir = depth + 1 < parts.len();
        let decided = matchers
            .iter()
            .rev()
            .map(|matcher| matcher.matched(&entry, is_dir))
            .find(|found| !found.is_none());
        if decided.is_some_and(|found| found.is_ignore()) {
            return true;
        }
    }
    false
}

/// The rules of the `.gitignore` file in `dir`; none when there is no such file.
fn gitignore(dir: &Path) -> Gitignore {
    let mut builder = GitignoreBuilder::new(dir);
    // A missing file adds nothing, and a line that does not parse is left out; the walk treats
    // both the same way.
    let _ = builder.add(dir.join(".gitignore"));
    builder.build().unwrap_or_else(|_| Gitignore::empty())
}

/// Whether [`walk`] would list the record path `path` (project-relative, with `/`) below `root`:
/// each folder of it is an entry of its parent folder that is a folder, and its file an entry
/// that is a file, each spelled exactly as given. A name in another case, a path through a file
/// and a symbolic link, which the walk does not follow, are not listed.
pub(crate) fn is_listed(root: &Path, path: &str) -> bool {
    let parts: Vec<&str> = path.split('/').collect();
    let mut dir: PathBuf = root.to_path_buf();
    for (depth, part) in parts.iter().enumerate() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            return false;
        };
        let is_dir = depth + 1 < parts.len();
        let listed = entries.filter_map(Result::ok).any(|entry| {
            entry.file_name() == **part
                && entry.file_type().is_ok_and(|kind| {
                    if is_dir {
                        kind.is_dir()
                    } else {
                        kind.is_file()
                    }
                })
        });
        if !listed {
            return false;
        }
        dir.push(part);
    }
    true
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

/// Walks `root` and returns its record files sorted by path. A file or folder that vanishes
/// during the walk is left out, as if the walk had come after its removal.
///
/// # Errors
///
/// Returns [`WalkError`] when the root or any entry that still exists cannot be read.
pub fn walk(root: &Path) -> Result<Vec<Walked>, WalkError> {
    collect(root, walker(root))
}

/// The walker of spec §5.7 over `root`.
fn walker(root: &Path) -> ignore::Walk {
    WalkBuilder::new(root)
        .hidden(true)
        .parents(false)
        .ignore(false)
        .git_ignore(true)
        .git_global(false)
        .git_exclude(false)
        .require_git(false)
        .follow_links(false)
        .build()
}

/// The record files among the `entries` of a walk of `root`, sorted by path.
fn collect(
    root: &Path,
    entries: impl IntoIterator<Item = Result<ignore::DirEntry, ignore::Error>>,
) -> Result<Vec<Walked>, WalkError> {
    let mut found = Vec::new();
    for entry in entries {
        let Some(entry) = keep(entry)? else {
            continue;
        };
        if !entry.file_type().is_some_and(|t| t.is_file()) {
            continue;
        }
        let Some(path) = relative(root, entry.path()) else {
            continue;
        };
        let Some(meta) = keep(entry.metadata())? else {
            continue;
        };
        found.push(Walked {
            path,
            size: meta.len(),
            mtime_ns: mtime_ns(&meta),
        });
    }
    found.sort_by(|a, b| a.path.as_str().cmp(b.path.as_str()));
    Ok(found)
}

/// The value of one step of the walk, or `None` when its entry vanished after its folder was
/// listed (a rename or delete during the walk): the file is no longer there, so it is skipped.
/// A missing root and every other error stay fatal.
fn keep<T>(step: Result<T, ignore::Error>) -> Result<Option<T>, WalkError> {
    match step {
        Ok(value) => Ok(Some(value)),
        Err(error) if vanished(&error) => Ok(None),
        Err(error) => Err(error.into()),
    }
}

/// Whether `error` says that an entry below the root no longer exists.
fn vanished(error: &ignore::Error) -> bool {
    error.depth().is_some_and(|depth| depth > 0)
        && error
            .io_error()
            .is_some_and(|io| io.kind() == std::io::ErrorKind::NotFound)
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
    fn is_ignored_agrees_with_the_walk() {
        let dir = tempfile::tempdir().expect("tempdir");
        let root = dir.path();
        let files = [
            "a.md",
            "x.log.md",
            "drafts/d.md",
            "drafts/keep.md",
            "sub/b.md",
            "sub/c.md",
            "sub/deep/x.md",
            "sub/deep/y.md",
            "w/y.md",
            "w/z.md",
            "kept/ignored-name.md",
            "kept/again.md",
        ];
        for rel in files {
            put(root, rel, "x\n");
        }
        // A folder rule, a `!` line under an ignored folder (never read, as the walk never
        // enters it), a glob, a deeper file that overrides and one that adds, a `!` line that
        // keeps one file of an ignored pattern, and a line that does not parse.
        put(
            root,
            ".gitignore",
            "drafts/\n!drafts/keep.md\n*.log.md\nc.md\nw/*\n!w/y.md\nignored-name.md\n[\n",
        );
        put(root, "drafts/.gitignore", "!keep.md\n");
        put(root, "sub/.gitignore", "!c.md\nb.md\ndeep/x.md\n");
        put(root, "kept/.gitignore", "!ignored-name.md\nagain.md\n");
        let walked: Vec<String> = walk(root)
            .expect("walk")
            .into_iter()
            .map(|w| w.path.as_str().to_owned())
            .collect();
        assert_eq!(
            walked,
            [
                "a.md",
                "kept/ignored-name.md",
                "sub/c.md",
                "sub/deep/y.md",
                "w/y.md"
            ]
        );
        for rel in files {
            assert_eq!(
                is_ignored(root, rel),
                !walked.iter().any(|w| w == rel),
                "{rel}"
            );
        }
    }

    #[test]
    fn is_listed_takes_the_names_on_disk() {
        let dir = tempfile::tempdir().expect("tempdir");
        let root = dir.path();
        put(root, "things/a.md", "x\n");
        assert!(is_listed(root, "things/a.md"));
        for rel in [
            "Things/a.md",
            "things/A.md",
            "things/b.md",
            "nowhere/a.md",
            "things/a.md/x.md",
            "things",
        ] {
            assert!(!is_listed(root, rel), "{rel}");
        }
        // A folder that cannot be listed lists nothing.
        assert!(!is_listed(&root.join("missing"), "a.md"));
    }

    #[cfg(unix)]
    #[test]
    fn is_listed_does_not_follow_symbolic_links() {
        let dir = tempfile::tempdir().expect("tempdir");
        let root = dir.path();
        put(root, "things/a.md", "x\n");
        std::os::unix::fs::symlink(root.join("things/a.md"), root.join("things/l.md"))
            .expect("symlink");
        std::os::unix::fs::symlink(root.join("things"), root.join("linked")).expect("symlink");
        assert!(!is_listed(root, "things/l.md"));
        assert!(!is_listed(root, "linked/a.md"));
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
            ("", false),
            (".md", false),
            ("../x.md", false),
            ("a/../x.md", false),
            ("/etc/x.md", false),
            ("C:/x.md", false),
            ("a\\b.md", false),
            ("a//b.md", false),
            ("./a.md", false),
            (".obsidian/x.md", false),
            ("lore/.hidden.md", false),
            ("lore/", false),
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

    /// An I/O error of `kind` at `depth`, as `ignore` reports one from `walkdir`.
    fn io_error(kind: std::io::ErrorKind, depth: usize) -> ignore::Error {
        ignore::Error::WithPath {
            path: "things/a.md".into(),
            err: Box::new(ignore::Error::WithDepth {
                depth,
                err: Box::new(ignore::Error::Io(std::io::Error::from(kind))),
            }),
        }
    }

    #[test]
    fn an_entry_that_vanished_during_the_walk_is_skipped() {
        let vanished: Result<u8, _> = Err(io_error(std::io::ErrorKind::NotFound, 1));
        assert!(matches!(keep(vanished), Ok(None)));
        assert!(matches!(keep(Ok::<u8, ignore::Error>(7)), Ok(Some(7))));
    }

    #[test]
    fn a_folder_that_vanished_during_the_walk_is_left_out() {
        let dir = tempfile::tempdir().expect("tempdir");
        put(dir.path(), "a.md", "x\n");
        let gone = std::iter::once(Err(io_error(std::io::ErrorKind::NotFound, 1)));
        let walked = collect(dir.path(), walker(dir.path()).chain(gone)).expect("walk");
        let paths: Vec<_> = walked.iter().map(|w| w.path.clone()).collect();
        assert_eq!(paths, vec![WalkedPath::Utf8("a.md".to_owned())]);
    }

    /// Only Unix reads the metadata of an entry after listing its folder; Windows gets it with
    /// the listing.
    #[cfg(unix)]
    #[test]
    fn a_file_deleted_before_its_metadata_is_read_is_left_out() {
        let dir = tempfile::tempdir().expect("tempdir");
        put(dir.path(), "a.md", "x\n");
        put(dir.path(), "b.md", "x\n");
        let doomed = dir.path().join("b.md");
        let entries = walker(dir.path()).inspect(|entry| {
            if entry.as_ref().is_ok_and(|e| e.path() == doomed) {
                fs::remove_file(&doomed).expect("remove");
            }
        });
        let walked = collect(dir.path(), entries).expect("walk");
        let paths: Vec<_> = walked.iter().map(|w| w.path.clone()).collect();
        assert_eq!(paths, vec![WalkedPath::Utf8("a.md".to_owned())]);
    }

    #[test]
    fn other_walk_errors_stay_fatal() {
        let errors = [
            io_error(std::io::ErrorKind::PermissionDenied, 1),
            io_error(std::io::ErrorKind::NotFound, 0),
            ignore::Error::Io(std::io::Error::from(std::io::ErrorKind::NotFound)),
        ];
        for error in errors {
            let shown = error.to_string();
            assert!(keep::<u8>(Err(error)).is_err(), "{shown}");
        }
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
