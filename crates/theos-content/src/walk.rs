//! Which files of a content project are records.
//!
//! The index walks the project with `ignore`:
//! - hidden files and folders are skipped the native way of the OS (Jann's ruling of
//!   2026-11-10): on Unix (Linux and macOS) a name that starts with `.` hides, on Windows the
//!   hidden attribute does, whatever the name;
//! - the folders the spec names (`.theos`, `.git`, `.obsidian`, `.vscode`, `.github`,
//!   `.worktrees`, `.trash`) are skipped by name, at any depth and on every OS;
//! - `.gitignore` files inside the project are honoured (never those of parent folders or the
//!   user's global one);
//! - P01's not-records (`README.md`, `templates/records/`, `tools/`, `attachments/`) are left
//!   out.
//!
//! The change boundary of `update`, `apply` and `preview` uses the same predicates, so a path
//! they take is one the walk lists.

use std::ffi::OsStr;
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

/// The folders that hold no records, by name, at any depth and on every OS (spec §5.7, P01 §5):
/// the cache, git, the editors' settings, the agent worktrees and Obsidian's trash.
const NAMED_FOLDERS: [&str; 7] = [
    ".theos",
    ".git",
    ".obsidian",
    ".vscode",
    ".github",
    ".worktrees",
    ".trash",
];

/// Whether `name` is one of the folders that hold no records.
fn is_named_folder(name: &str) -> bool {
    NAMED_FOLDERS.contains(&name)
}

/// Whether a project-relative path (with `/`) is a record by its spelling, the same on every OS:
/// a plain relative path (not empty, not starting with `/`, no `\` or `:`, no empty, `.` or `..`
/// component) that is under none of the folders `.theos`, `.git`, `.obsidian`, `.vscode`,
/// `.github`, `.worktrees` and `.trash` (at any depth), whose file name ends in `.md` after at
/// least one more character and is not `README.md`, and that is not under `templates/records/`,
/// `tools/` or `attachments/`.
///
/// Whether a path is hidden depends on the OS and, on Windows, on the disk, so this check leaves
/// it out: [`walk`] skips a hidden entry on top of it, on Unix one whose name starts with `.`
/// and on Windows one with the hidden attribute.
#[must_use]
pub fn is_record_path(path: &str) -> bool {
    let plain = !path.contains(['\\', ':'])
        && path
            .split('/')
            .all(|part| !part.is_empty() && part != "." && part != "..");
    let mut parts = path.split('/');
    let name = parts.next_back().unwrap_or(path);
    let folders_hold_records = parts.all(|folder| !is_named_folder(folder));
    plain
        && folders_hold_records
        && name
            .strip_suffix(".md")
            .is_some_and(|stem| !stem.is_empty())
        && name != "README.md"
        && !["templates/records/", "tools/", "attachments/"]
            .iter()
            .any(|prefix| path.starts_with(prefix))
}

/// Whether the record path `path` (project-relative, with `/`) below `root` is hidden, so that
/// [`walk`] does not list it: a folder of it or its file is hidden on this OS (see
/// [`is_hidden_entry`]). On Windows only what exists on disk can carry the attribute, so a path
/// that is not on disk is hidden only when a folder of it that exists is.
pub(crate) fn is_hidden(root: &Path, path: &str) -> bool {
    let mut entry = root.to_path_buf();
    path.split('/').any(|part| {
        entry.push(part);
        is_hidden_entry(&entry)
    })
}

/// Whether the walk skips the entry at `path` and, for a folder, everything below it: it is
/// hidden on this OS, or it is one of the folders that hold no records.
fn is_skipped_entry(path: &Path) -> bool {
    path.file_name()
        .and_then(OsStr::to_str)
        .is_some_and(is_named_folder)
        || is_hidden_entry(path)
}

/// Whether the file or folder at `path` is hidden on Unix: its name starts with `.`. Nothing is
/// read from disk.
#[cfg(unix)]
fn is_hidden_entry(path: &Path) -> bool {
    path.file_name()
        .is_some_and(|name| name.as_encoded_bytes().starts_with(b"."))
}

/// Whether the file or folder at `path` is hidden on Windows: it exists and carries the hidden
/// attribute (`FILE_ATTRIBUTE_HIDDEN`), whatever its name. A symbolic link is judged by its own
/// attributes, as the walk sees it.
#[cfg(windows)]
fn is_hidden_entry(path: &Path) -> bool {
    use std::os::windows::fs::MetadataExt;
    /// `FILE_ATTRIBUTE_HIDDEN` of the Windows API.
    const HIDDEN: u32 = 0x2;
    std::fs::symlink_metadata(path).is_ok_and(|meta| meta.file_attributes() & HIDDEN != 0)
}

/// Whether the `.gitignore` files inside the project at `root` keep the record path `path`
/// (project-relative, with `/`) from [`walk`]. As in the walk, each folder of the path and then
/// the file are matched against the `.gitignore` of every folder above them, the deepest first,
/// and the first file that has a rule for them decides; an ignored folder keeps everything below
/// it out, also a file that a `!` line names. Lines that do not parse are skipped, as the walk
/// skips them.
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

/// Walks `root` and returns its record files sorted by path (see [`is_record_path`]). It does not
/// enter a hidden folder or list a hidden file, the native way of the OS: on Unix (Linux and
/// macOS) a name that starts with `.` hides, on Windows the hidden attribute does, whatever the
/// name. On every OS it does not enter the folders `.theos`, `.git`, `.obsidian`, `.vscode`,
/// `.github`, `.worktrees` and `.trash`, at any depth. A file or folder that vanishes during the
/// walk is left out, as if the walk had come after its removal.
///
/// # Errors
///
/// Returns [`WalkError`] when the root or any entry that still exists cannot be read.
pub fn walk(root: &Path) -> Result<Vec<Walked>, WalkError> {
    collect(root, walker(root))
}

/// The walker of spec §5.7 over `root`. `ignore`'s own hidden rule is off: on Windows it hides a
/// name that starts with `.` too, and the change boundary must decide the same way as the walk.
fn walker(root: &Path) -> ignore::Walk {
    WalkBuilder::new(root)
        .hidden(false)
        .parents(false)
        .ignore(false)
        .git_ignore(true)
        .git_global(false)
        .git_exclude(false)
        .require_git(false)
        .follow_links(false)
        .filter_entry(|entry| !is_skipped_entry(entry.path()))
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

    /// Gives the file or folder `rel` below `root` the hidden attribute, as `attrib +h` does.
    #[cfg(windows)]
    fn hide(root: &Path, rel: &str) {
        let path = rel
            .split('/')
            .fold(root.to_path_buf(), |path, part| path.join(part));
        let status = std::process::Command::new("attrib")
            .arg("+h")
            .arg(&path)
            .status()
            .expect("run attrib");
        assert!(status.success(), "attrib +h {}", path.display());
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
            ".git/x.md",
            ".vscode/x.md",
            ".github/x.md",
            ".worktrees/w/z.md",
            "world/.trash/y.md",
            "world/deep/.obsidian/x.md",
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

    /// The change boundary of `update`, `apply` and `preview` (`is_record_path`, `is_hidden`,
    /// `is_ignored`) keeps exactly the files that the walk lists.
    #[test]
    fn the_change_boundary_agrees_with_the_walk() {
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
            // Hidden on Unix by their names; on Windows only with the attribute.
            ".drafts/a.md",
            "lore/.hidden.md",
            // Hidden on Windows below (the file, and the folder), never on Unix.
            "things/b.md",
            "secret/s.md",
            // Never records, on every OS.
            ".obsidian/x.md",
            "lore/.trash/y.md",
            ".worktrees/w/z.md",
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
        #[cfg(unix)]
        let native = ["secret/s.md", "things/b.md"];
        #[cfg(windows)]
        let native = {
            hide(root, "things/b.md");
            hide(root, "secret");
            [".drafts/a.md", "lore/.hidden.md"]
        };
        let walked: Vec<String> = walk(root)
            .expect("walk")
            .into_iter()
            .map(|w| w.path.as_str().to_owned())
            .collect();
        let mut want = vec![
            "a.md",
            "kept/ignored-name.md",
            "sub/c.md",
            "sub/deep/y.md",
            "w/y.md",
        ];
        want.extend(native);
        want.sort_unstable();
        assert_eq!(walked, want);
        for rel in files {
            let kept = is_record_path(rel) && !is_hidden(root, rel) && !is_ignored(root, rel);
            assert_eq!(kept, walked.iter().any(|w| w == rel), "{rel}");
        }
    }

    /// On Unix a name that starts with `.` hides a file or folder, whether or not it exists.
    #[cfg(unix)]
    #[test]
    fn a_dot_name_hides_on_unix() {
        let dir = tempfile::tempdir().expect("tempdir");
        let root = dir.path();
        put(root, ".drafts/a.md", "x\n");
        put(root, "lore/.hidden.md", "x\n");
        put(root, "things/b.md", "x\n");
        for rel in [
            ".drafts/a.md",
            "lore/.hidden.md",
            ".new/x.md",
            "lore/.new.md",
        ] {
            assert!(is_hidden(root, rel), "{rel}");
        }
        for rel in ["things/b.md", "things/new.md"] {
            assert!(!is_hidden(root, rel), "{rel}");
        }
    }

    /// On Windows the hidden attribute of a file or of one of its folders hides it; a name that
    /// starts with `.` does not, and a path that is not on disk is not hidden unless a folder of
    /// it that exists is.
    #[cfg(windows)]
    #[test]
    fn the_hidden_attribute_hides_on_windows() {
        let dir = tempfile::tempdir().expect("tempdir");
        let root = dir.path();
        for rel in [".drafts/a.md", "things/b.md", "things/c.md", "secret/s.md"] {
            put(root, rel, "x\n");
        }
        hide(root, "things/b.md");
        hide(root, "secret");
        for rel in ["things/b.md", "secret/s.md", "secret/new.md"] {
            assert!(is_hidden(root, rel), "{rel}");
        }
        for rel in [".drafts/a.md", "things/c.md", "things/new.md", ".new/x.md"] {
            assert!(!is_hidden(root, rel), "{rel}");
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
            ("a/.md", false),
            ("lore/", false),
            // Hidden or not is the OS's and the disk's matter, not the spelling's.
            ("lore/.hidden.md", true),
            (".drafts/a.md", true),
            ("lore/.trash.md", true),
            // The folders the spec names are never records, at any depth.
            (".theos/cache/x.md", false),
            (".git/x.md", false),
            (".obsidian/x.md", false),
            (".vscode/x.md", false),
            (".github/x.md", false),
            (".worktrees/w/z.md", false),
            (".trash/x.md", false),
            ("lore/.trash/y.md", false),
            ("world/deep/.obsidian/x.md", false),
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
