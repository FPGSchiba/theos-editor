//! A file's bytes at a version: `HEAD`, the index, any revision, or the working tree.

use std::path::Path;

use crate::git::{Git, GitError, args_text};
use crate::read_args::read_args;
use crate::staged::{self, blobs, parse, regular_mode};

/// Where a file's bytes come from.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Version<'a> {
    /// The commit `HEAD` names.
    Head,
    /// The index, stage 0.
    Index,
    /// Any revision git accepts: a commit id, a branch such as `agent/s-04`.
    Rev(&'a str),
    /// The file on disk.
    WorkTree,
}

/// What a version holds at a path.
#[non_exhaustive]
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Content {
    /// The version holds nothing at the path (or there is no commit yet).
    Absent,
    /// A regular file and its bytes.
    File(Vec<u8>),
    /// A link, a submodule or a folder.
    NotAFile,
}

/// Reads the file at `path` (git's bytes: relative to `top`, with `/`, UTF-8) at `version`.
///
/// A path with `*`, `?`, `[`, `:` or a leading `-` is a path, never a pattern or an option. An
/// intent-to-add entry (`git add -N`) reads as [`Content::Absent`] from the index, as a commit
/// does not record it. A path the version lacks, and every path of `HEAD` before the first commit, read as
/// `Absent`; a revision that does not exist is an error with git's message. The bytes are the stored ones: no line-ending change.
///
/// # Errors
///
/// The errors of [`Git::run`], [`GitError::Output`] when an answer of git cannot be read or the
/// path is not UTF-8, and [`GitError::Io`] when the working-tree file cannot be read.
pub fn read(git: &Git, top: &Path, version: Version<'_>, path: &[u8]) -> Result<Content, GitError> {
    let path = std::str::from_utf8(path).map_err(|_| GitError::Output {
        args: vec!["rev-parse".to_owned()],
        message: format!(
            "the path `{}` is not UTF-8; theos reads only UTF-8 paths",
            String::from_utf8_lossy(path)
        ),
    })?;
    match version {
        Version::WorkTree => work_tree(top, path),
        Version::Index => index(git, top, path),
        Version::Head | Version::Rev(_) => commit(git, top, version, path),
    }
}

fn work_tree(top: &Path, path: &str) -> Result<Content, GitError> {
    work_tree_with(top, path, |file| std::fs::read(file))
}

/// [`work_tree`] with the read of the bytes as a parameter, so a test can make a file vanish
/// between the metadata and the read.
fn work_tree_with(
    top: &Path,
    path: &str,
    read: impl FnOnce(&Path) -> std::io::Result<Vec<u8>>,
) -> Result<Content, GitError> {
    let file = top.join(path);
    let io = |source| GitError::Io { source };
    match std::fs::symlink_metadata(&file) {
        Ok(meta) if meta.is_file() => match read(&file) {
            Ok(bytes) => Ok(Content::File(bytes)),
            // The file vanished between the metadata and the read.
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(Content::Absent),
            Err(error) => Err(io(error)),
        },
        Ok(_) => Ok(Content::NotAFile),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(Content::Absent),
        Err(error) => Err(io(error)),
    }
}

/// The object `spec` (`<rev>:<path>`, `:0:<path>`) names, or `None` when git says there is none:
/// exit code 1 with nothing on stderr, as `--quiet` makes it.
fn object(git: &Git, top: &Path, spec: &str) -> Result<Option<String>, GitError> {
    let args = read_args(
        "rev-parse",
        &["--verify", "--quiet", "--end-of-options", spec],
    );
    match git.run(&args, top, None, &[]) {
        Ok(out) => String::from_utf8(out)
            .map(|text| Some(text.trim().to_owned()))
            .map_err(|_| GitError::Output {
                args: args_text(&args),
                message: "the object id is not UTF-8".to_owned(),
            }),
        Err(GitError::Failed {
            code: Some(1),
            stderr,
            ..
        }) if stderr.is_empty() => Ok(None),
        Err(error) => Err(error),
    }
}

/// The bytes of `object` when `mode` is a regular file, else [`Content::NotAFile`].
fn bytes(git: &Git, top: &Path, mode: u32, object: &str) -> Result<Content, GitError> {
    if !regular_mode(mode) {
        return Ok(Content::NotAFile);
    }
    let mut blob = blobs(git, top, &[object])?;
    blob.pop()
        .map(Content::File)
        .ok_or_else(|| GitError::Output {
            args: vec!["cat-file".to_owned(), "--batch".to_owned()],
            message: "no object came back".to_owned(),
        })
}

/// A file of a commit: `ls-tree` of its folder gives the mode, `cat-file` the bytes.
///
/// The mode is read without a pathspec, so a name git would take for something else (on Windows
/// `1:notes.md`, read as a drive) is still found: the entry is matched by its exact name.
fn commit(git: &Git, top: &Path, version: Version<'_>, path: &str) -> Result<Content, GitError> {
    let rev = match version {
        Version::Rev(rev) => rev,
        _ => "HEAD",
    };
    let Some(object) = object(git, top, &format!("{rev}:{path}"))? else {
        if let Version::Rev(rev) = version {
            // A path an existing revision lacks is absent; a revision that does not exist is an
            // error with git's own message.
            let commit = format!("{rev}^{{commit}}");
            git.run(
                &read_args("rev-parse", &["--verify", "--end-of-options", &commit]),
                top,
                None,
                &[],
            )?;
        }
        return Ok(Content::Absent);
    };
    let (folder, name) = match path.rsplit_once('/') {
        Some((folder, name)) => (format!("{rev}:{folder}"), name),
        None => (format!("{rev}^{{tree}}"), path),
    };
    let mode = staged::query(
        git,
        top,
        "ls-tree",
        &["-z", "--end-of-options", &folder],
        None,
        |out| entry_mode(out, name),
    )?;
    bytes(git, top, mode, &object)
}

/// The mode of the entry called `name` in `ls-tree -z` output: `<mode> SP <type> SP <oid> TAB
/// <name> NUL` per entry.
fn entry_mode(out: &[u8], name: &str) -> Result<u32, String> {
    for record in out.split(|byte| *byte == 0) {
        let Some(tab) = record.iter().position(|byte| *byte == b'\t') else {
            continue;
        };
        if &record[tab + 1..] == name.as_bytes() {
            let head = std::str::from_utf8(&record[..tab]).map_err(|_| "an entry is not UTF-8")?;
            let (mode, _) = head.split_once(' ').ok_or("an entry has no mode")?;
            return u32::from_str_radix(mode, 8)
                .map_err(|_| format!("`{mode}` is not an octal mode"));
        }
    }
    Err(format!("the folder has no entry called {name}"))
}

/// A file of the index at stage 0: `ls-files -s` gives the mode, `cat-file` the bytes.
fn index(git: &Git, top: &Path, path: &str) -> Result<Content, GitError> {
    let Some(object) = object(git, top, &format!(":0:{path}"))? else {
        return Ok(Content::Absent);
    };
    if staged::is_intent_to_add(git, top, path)? {
        return Ok(Content::Absent);
    }
    let entries = staged::query(
        git,
        top,
        "ls-files",
        &["-s", "-z", "--", path],
        None,
        parse::ls_files,
    )?;
    let entry = entries
        .iter()
        .find(|entry| entry.path == path.as_bytes())
        .ok_or_else(|| GitError::Output {
            args: vec!["ls-files".to_owned()],
            message: format!("the index has no entry for {path}"),
        })?;
    bytes(git, top, entry.mode, &object)
}

#[cfg(test)]
mod tests {
    use std::io::{Error, ErrorKind};

    use super::*;

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    // A file that vanishes between the metadata and the read is absent, not an error. The OS cannot
    // make that race deterministic, so the read is a seam that fails as a vanished file does.
    #[test]
    fn a_file_gone_before_the_read_is_absent() -> TestResult {
        let dir = tempfile::tempdir()?;
        std::fs::write(dir.path().join("a.md"), b"x")?;
        let content = work_tree_with(dir.path(), "a.md", |_| {
            Err(Error::from(ErrorKind::NotFound))
        })?;
        assert_eq!(content, Content::Absent);
        Ok(())
    }

    #[test]
    fn another_read_error_stays_an_error() -> TestResult {
        let dir = tempfile::tempdir()?;
        std::fs::write(dir.path().join("a.md"), b"x")?;
        let content = work_tree_with(dir.path(), "a.md", |_| {
            Err(Error::from(ErrorKind::PermissionDenied))
        });
        assert!(matches!(content, Err(GitError::Io { .. })), "{content:?}");
        Ok(())
    }

    #[test]
    fn a_readable_file_is_its_bytes() -> TestResult {
        let dir = tempfile::tempdir()?;
        std::fs::write(dir.path().join("a.md"), b"x")?;
        assert_eq!(work_tree(dir.path(), "a.md")?, Content::File(b"x".to_vec()));
        Ok(())
    }

    #[test]
    fn an_entry_is_found_by_its_exact_name() {
        let out = b"100644 blob aaaa\ta.md\x00120000 blob bbbb\tb.md\0";
        assert_eq!(entry_mode(out, "b.md"), Ok(0o120_000));
        assert_eq!(entry_mode(out, "a.md"), Ok(0o100_644));
    }

    #[test]
    fn a_name_the_folder_lacks_is_an_error() {
        let error = entry_mode(b"100644 blob aaaa\ta.md\0", "c.md").err();
        assert_eq!(
            error.as_deref(),
            Some("the folder has no entry called c.md")
        );
    }

    #[test]
    fn a_record_without_a_tab_is_skipped_and_a_broken_entry_is_an_error() {
        let out = b"stray\x00100644 blob aaaa\ta.md\0";
        assert_eq!(entry_mode(out, "a.md"), Ok(0o100_644));
        assert!(entry_mode(b"100644\ta.md\0", "a.md").is_err());
        assert!(entry_mode(b"9 blob aaaa\ta.md\0", "a.md").is_err());
        assert!(entry_mode(b"\xff \ta.md\0", "a.md").is_err());
    }

    #[test]
    fn a_path_that_is_not_utf_8_is_an_output_error() -> TestResult {
        let dir = tempfile::tempdir()?;
        let git = Git::find()?;
        for version in [Version::WorkTree, Version::Index, Version::Head] {
            let result = read(&git, dir.path(), version, &[0xff, b'.', b'm', b'd']);
            let error = result.err().ok_or("expected an error")?;
            assert!(matches!(error, GitError::Output { .. }), "{error:?}");
            assert_eq!(
                error.to_string(),
                "git rev-parse gave an unusable answer: the path `\u{fffd}.md` is not UTF-8; \
                 theos reads only UTF-8 paths"
            );
        }
        Ok(())
    }
}
