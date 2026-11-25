//! Whether a relative path names a file the way the disk spells it.
//!
//! A file looked up by a project-relative or binaries-relative path counts only when the file on
//! disk is spelled exactly so, so that case-insensitive file systems (Windows, macOS) give the
//! same verdict as Linux. Such a file system opens `boards/ok.bin` for `Boards/OK.bin`, and
//! Windows also opens a file by its short (8.3) name or with a trailing dot or space. [`of`] tells
//! these apart from the exact name by the listing of each folder on the way.
//!
//! Every lookup by relative path decides through it: the records and input files of the index
//! (`walk::is_listed`), binary references, the formalism mirror, and the part on disk of a path
//! that the command boundary of `theos-studio` judges (`walk::disk_spelling`).

use std::ffi::OsString;
use std::fs::DirEntry;
use std::io;
use std::path::{Path, PathBuf};

use same_file::Handle;

/// How the disk spells a relative path below a root.
#[derive(Debug)]
pub(crate) enum Spelling {
    /// Each segment is an entry of its folder under exactly its name: the entries, outermost
    /// first.
    Exact(Vec<DirEntry>),
    /// The file system finds the path, but a segment only under another name: the path as the
    /// folders list it, with `/`.
    Other(String),
    /// A segment is no entry of its folder under any name, or a folder on the way cannot be
    /// listed: why. A segment that the file system finds although no entry of its folder is the
    /// one it opens (such as a device name on Windows) is `NotFound` too.
    Absent(io::Error),
}

impl Spelling {
    /// Whether this is the spelling of a file that the walk of the index would list: exact,
    /// each folder a folder and the last entry a file, none of them a symbolic link.
    pub(crate) fn is_plain_file(&self) -> bool {
        let Spelling::Exact(entries) = self else {
            return false;
        };
        let folders = entries.len().saturating_sub(1);
        !entries.is_empty()
            && entries.iter().enumerate().all(|(depth, entry)| {
                entry.file_type().is_ok_and(|kind| {
                    if depth < folders {
                        kind.is_dir()
                    } else {
                        kind.is_file()
                    }
                })
            })
    }
}

/// How the disk spells `segments` below `root`.
///
/// Each segment is looked up in the listing of the folder before it ([`std::fs::read_dir`], which
/// follows a link to a folder). A segment that the listing lacks under exactly its name, but that
/// the file system still finds, is matched to the entry the file system opens for it, and the
/// lookup goes on below that entry, so that [`Spelling::Other`] gives the whole path as the disk
/// spells it.
pub(crate) fn of(root: &Path, segments: &[&str]) -> Spelling {
    let mut dir: PathBuf = root.to_path_buf();
    let mut entries = Vec::with_capacity(segments.len());
    let mut on_disk = Vec::with_capacity(segments.len());
    let mut exact = true;
    for segment in segments {
        let listing = match std::fs::read_dir(&dir) {
            Ok(listing) => listing,
            Err(error) => return Spelling::Absent(error),
        };
        let name = if let Some(entry) = listing
            .filter_map(Result::ok)
            .find(|entry| entry.file_name() == **segment)
        {
            entries.push(entry);
            OsString::from(segment)
        } else {
            match opened_for(&dir, segment) {
                Ok(name) => {
                    exact = false;
                    name
                }
                Err(error) => return Spelling::Absent(error),
            }
        };
        on_disk.push(name.to_string_lossy().into_owned());
        dir.push(name);
    }
    if exact {
        Spelling::Exact(entries)
    } else {
        Spelling::Other(on_disk.join("/"))
    }
}

/// The name of the entry of `dir` that the file system opens for `segment`, which the listing of
/// `dir` lacks: the entry that is the same file ([`same_file::Handle`]: device and inode on Unix,
/// volume and file index on Windows) and that is a symbolic link exactly when the entry opened
/// for `segment` is one. Several such entries are links to one file; the first by name is taken.
///
/// # Errors
///
/// The error of the file system when it finds nothing for `segment` or cannot open it, and
/// `NotFound` when no entry of `dir` is the one it opens.
fn opened_for(dir: &Path, segment: &str) -> io::Result<OsString> {
    let wanted = dir.join(segment);
    let is_link = std::fs::symlink_metadata(&wanted)?.file_type().is_symlink();
    let file = Handle::from_path(&wanted)?;
    std::fs::read_dir(dir)?
        .filter_map(Result::ok)
        .filter(|entry| {
            entry
                .file_type()
                .is_ok_and(|kind| kind.is_symlink() == is_link)
        })
        .filter(|entry| Handle::from_path(entry.path()).is_ok_and(|other| other == file))
        .map(|entry| entry.file_name())
        .min()
        .ok_or_else(|| io::Error::from(io::ErrorKind::NotFound))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn put(root: &Path, rel: &str) {
        let path = root.join(rel);
        std::fs::create_dir_all(path.parent().expect("has a parent")).expect("mkdir");
        std::fs::write(path, "x\n").expect("write");
    }

    /// The names of the entries of an exact spelling.
    fn names(spelling: &Spelling) -> Vec<String> {
        match spelling {
            Spelling::Exact(entries) => entries
                .iter()
                .map(|entry| entry.file_name().to_string_lossy().into_owned())
                .collect(),
            other => panic!("not exact: {other:?}"),
        }
    }

    #[test]
    fn the_exact_name_is_exact() {
        let dir = tempfile::tempdir().expect("tempdir");
        let root = dir.path();
        put(root, "boards/ok.bin");
        let found = of(root, &["boards", "ok.bin"]);
        assert_eq!(names(&found), ["boards", "ok.bin"]);
        assert!(found.is_plain_file());
        let folder = of(root, &["boards"]);
        assert_eq!(names(&folder), ["boards"]);
        assert!(!folder.is_plain_file(), "a folder is no file");
    }

    /// On Linux `OK.bin` does not exist at all; on Windows and macOS the file system opens
    /// `ok.bin` for it. Neither is the file of exactly that name.
    #[test]
    fn a_name_in_another_case_is_not_as_spelled() {
        let dir = tempfile::tempdir().expect("tempdir");
        let root = dir.path();
        put(root, "boards/ok.bin");
        let folds_case = root.join("BOARDS").join("OK.BIN").exists();
        for wanted in [
            ["boards", "OK.bin"],
            ["Boards", "ok.bin"],
            ["BOARDS", "OK.BIN"],
        ] {
            let found = of(root, &wanted);
            assert!(!found.is_plain_file(), "{wanted:?}");
            match found {
                Spelling::Other(on_disk) if folds_case => assert_eq!(on_disk, "boards/ok.bin"),
                Spelling::Absent(error) if !folds_case => {
                    assert_eq!(error.kind(), io::ErrorKind::NotFound, "{wanted:?}");
                }
                other => panic!("{wanted:?}, folds case {folds_case}: {other:?}"),
            }
        }
    }

    #[test]
    fn a_path_without_a_file_is_absent() {
        let dir = tempfile::tempdir().expect("tempdir");
        let root = dir.path();
        put(root, "boards/ok.bin");
        for wanted in [
            &["boards", "none.bin"][..],
            &["nowhere", "ok.bin"],
            &["Boards", "none.bin"],
            &["boards", "ok.bin", "x.bin"],
            &[""],
        ] {
            let found = of(root, wanted);
            assert!(
                matches!(found, Spelling::Absent(_)),
                "{wanted:?}: {found:?}"
            );
        }
        // A root that does not exist lists nothing.
        let found = of(&root.join("missing"), &["ok.bin"]);
        assert!(
            matches!(&found, Spelling::Absent(error) if error.kind() == io::ErrorKind::NotFound),
            "{found:?}"
        );
    }

    /// The entry opened for a name is found by file identity, on every OS: the file itself, the
    /// first by name of several hard links to it, and nothing for a name the file system does
    /// not find or that names no entry of the folder (the folder itself).
    #[test]
    fn the_entry_opened_for_a_name_is_the_same_file() {
        let dir = tempfile::tempdir().expect("tempdir");
        let root = dir.path();
        put(root, "boards/ok.bin");
        put(root, "boards/other.bin");
        let boards = root.join("boards");
        assert_eq!(opened_for(&boards, "ok.bin").expect("found"), "ok.bin");
        std::fs::hard_link(boards.join("ok.bin"), boards.join("aa.bin")).expect("hard link");
        assert_eq!(opened_for(&boards, "ok.bin").expect("found"), "aa.bin");
        for nothing in ["none.bin", ""] {
            let error = opened_for(&boards, nothing).expect_err(nothing);
            assert_eq!(error.kind(), io::ErrorKind::NotFound, "{nothing:?}");
        }
    }

    /// Windows opens a file for its name with a trailing dot or space.
    #[cfg(windows)]
    #[test]
    fn a_trailing_dot_or_space_is_another_spelling_on_windows() {
        let dir = tempfile::tempdir().expect("tempdir");
        let root = dir.path();
        put(root, "boards/ok.bin");
        for wanted in [
            ["boards", "ok.bin."],
            ["boards", "ok.bin "],
            ["boards.", "ok.bin"],
        ] {
            let found = of(root, &wanted);
            assert!(
                matches!(&found, Spelling::Other(on_disk) if on_disk == "boards/ok.bin"),
                "{wanted:?}: {found:?}"
            );
        }
    }

    /// A link is listed under its own name; a lookup goes on below a link to a folder, and only
    /// [`Spelling::is_plain_file`] refuses it.
    #[cfg(unix)]
    #[test]
    fn a_link_is_an_entry_of_its_own_name() {
        let dir = tempfile::tempdir().expect("tempdir");
        let root = dir.path();
        put(root, "boards/ok.bin");
        std::os::unix::fs::symlink(root.join("boards/ok.bin"), root.join("boards/link.bin"))
            .expect("symlink");
        std::os::unix::fs::symlink(root.join("boards"), root.join("linked")).expect("symlink");
        for wanted in [["boards", "link.bin"], ["linked", "ok.bin"]] {
            let found = of(root, &wanted);
            assert_eq!(names(&found), wanted);
            assert!(!found.is_plain_file(), "{wanted:?}");
        }
    }
}
