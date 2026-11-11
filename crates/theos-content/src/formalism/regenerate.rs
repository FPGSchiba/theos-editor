//! Writing the equation records of a project from its source.

use std::path::{Path, PathBuf};

use super::{FOLDER, SOURCE, SourceError, parse, render};
use crate::spelling::{self, Spelling};

/// What a regeneration did, as project-relative paths with `/`, in id order.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Regenerated {
    /// The files written: new, or with other bytes.
    pub written: Vec<String>,
    /// The files that held the rendered bytes already.
    pub unchanged: Vec<String>,
}

/// Why a regeneration failed.
#[derive(Debug, thiserror::Error)]
pub enum FormalismError {
    /// The project has no file `canon/formalism/_source.json`: none of exactly that name, or a
    /// folder or symbolic link, which the index does not take for the source either.
    #[error("the project has no {SOURCE}")]
    Missing,
    /// The source is not valid or cannot be rendered.
    #[error("{SOURCE} is not valid: {0}")]
    Source(#[from] SourceError),
    /// A file could not be read or written.
    #[error("{path}: {source}")]
    Io {
        /// The file or folder.
        path: PathBuf,
        /// The cause.
        source: std::io::Error,
    },
    /// A folder or record the regeneration would write through is a symbolic link, or lies
    /// outside the project once links are resolved. Nothing is written.
    #[error(
        "{path} is a symbolic link or lies outside the project, so nothing is written through it"
    )]
    Outside {
        /// The file or folder.
        path: PathBuf,
    },
    /// The file system finds the source or a record only under another spelling, as a
    /// case-insensitive one (Windows, macOS) does. Only the file of exactly that name counts, so
    /// the other file is neither read nor written, and nothing is written.
    #[error(
        "the file system takes {on_disk} for {path}, which counts only when spelled exactly so; \
         nothing is written until {on_disk} is renamed or moved"
    )]
    Spelling {
        /// The project-relative path, with `/`.
        path: String,
        /// The file that the file system finds for it, project-relative with `/`.
        on_disk: String,
    },
}

/// Writes each equation record of `<root>/canon/formalism/_source.json` whose bytes differ from
/// its rendering (LF). It writes nothing else and never deletes: a record
/// the source does not list stays, for `formalism/stray` to report. It never writes through a
/// symbolic link: when `canon`, `canon/formalism` or a record it would write is one, or resolves
/// outside the project, it writes nothing.
///
/// Like the index, it takes the source and the records only under exactly their names. A source
/// that is a link or a folder is no source. Where the file system finds the source or a record
/// only under another spelling, as a case-insensitive one (Windows, macOS) does, it reads and
/// writes nothing and names both spellings; elsewhere a record of another spelling is another
/// file.
///
/// # Errors
///
/// [`FormalismError`] when there is no source, it is not valid, a folder or record is a link
/// or lies outside the project, the source or a record is spelled otherwise on disk, or a file
/// cannot be read or written.
pub fn regenerate(root: &Path) -> Result<Regenerated, FormalismError> {
    let mut folder = root.to_path_buf();
    for part in FOLDER.split('/') {
        folder.push(part);
        refuse_link(&folder)?;
    }
    let bytes = read_source(root)?;
    let text = String::from_utf8(bytes)
        .map_err(|_| SourceError::format("the file is not UTF-8".to_owned()))?;
    let files = render(&parse(&text)?)?;
    // The source was read from the folder, so the project and the folder exist.
    let project = std::fs::canonicalize(root).map_err(|error| io(root.to_path_buf(), error))?;
    refuse_outside(&project, &folder)?;
    // Every record is judged before the first write, so a refused one leaves all as they were.
    let mut done = Regenerated::default();
    let mut writes = Vec::new();
    for (path, text) in files {
        refuse_other_spelling(root, &path)?;
        let file = root.join(&path);
        refuse_link(&file)?;
        if file.exists() {
            refuse_outside(&project, &file)?;
        }
        if std::fs::read(&file).is_ok_and(|held| held == text.as_bytes()) {
            done.unchanged.push(path);
        } else {
            writes.push((path, file, text));
        }
    }
    for (path, file, text) in writes {
        std::fs::write(&file, text).map_err(|error| io(file, error))?;
        done.written.push(path);
    }
    Ok(done)
}

/// The bytes of the source, read only when it is the file of exactly its name, as the index
/// takes it ([`Spelling::is_plain_file`]).
fn read_source(root: &Path) -> Result<Vec<u8>, FormalismError> {
    let path = root.join(SOURCE);
    let segments: Vec<&str> = SOURCE.split('/').collect();
    match spelling::of(root, &segments) {
        found if found.is_plain_file() => std::fs::read(&path).map_err(|error| {
            if error.kind() == std::io::ErrorKind::NotFound {
                FormalismError::Missing
            } else {
                io(path, error)
            }
        }),
        Spelling::Other(on_disk) => Err(FormalismError::Spelling {
            path: SOURCE.to_owned(),
            on_disk,
        }),
        Spelling::Absent(error) if error.kind() != std::io::ErrorKind::NotFound => {
            Err(io(path, error))
        }
        // None, or a folder or link of that name: the index sees no source either.
        Spelling::Exact(_) | Spelling::Absent(_) => Err(FormalismError::Missing),
    }
}

/// Refuses the project-relative `path` when the file system finds it only under another
/// spelling: writing it would write into that other file.
fn refuse_other_spelling(root: &Path, path: &str) -> Result<(), FormalismError> {
    let segments: Vec<&str> = path.split('/').collect();
    match spelling::of(root, &segments) {
        Spelling::Other(on_disk) => Err(FormalismError::Spelling {
            path: path.to_owned(),
            on_disk,
        }),
        // A record that is not there is written new; one that cannot be looked up fails at its
        // read or write.
        Spelling::Exact(_) | Spelling::Absent(_) => Ok(()),
    }
}

/// Refuses `path` when it is a symbolic link; a path that does not exist is fine.
fn refuse_link(path: &Path) -> Result<(), FormalismError> {
    match std::fs::symlink_metadata(path) {
        Ok(meta) if meta.file_type().is_symlink() => Err(outside(path)),
        Ok(_) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(io(path.to_path_buf(), error)),
    }
}

/// Refuses `path` when, with links resolved, it does not lie under `project` (resolved too).
fn refuse_outside(project: &Path, path: &Path) -> Result<(), FormalismError> {
    let real = std::fs::canonicalize(path).map_err(|error| io(path.to_path_buf(), error))?;
    if real.starts_with(project) {
        Ok(())
    } else {
        Err(outside(path))
    }
}

fn outside(path: &Path) -> FormalismError {
    FormalismError::Outside {
        path: path.to_path_buf(),
    }
}

fn io(path: PathBuf, source: std::io::Error) -> FormalismError {
    FormalismError::Io { path, source }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SOURCE_TEXT: &str = r#"{"format": 1, "page": "https://example.invalid/p", "equations": [
        {"id": "E1", "title": "One", "anchor": "https://example.invalid/p#1", "text": "a"},
        {"id": "E2", "title": "Two", "anchor": "https://example.invalid/p#2", "text": "b"}]}"#;

    fn project(source: &str) -> Result<tempfile::TempDir, std::io::Error> {
        let dir = tempfile::tempdir()?;
        std::fs::create_dir_all(dir.path().join(FOLDER))?;
        std::fs::write(dir.path().join(SOURCE), source)?;
        Ok(dir)
    }

    #[test]
    fn a_missing_source_is_an_error() -> Result<(), std::io::Error> {
        let dir = tempfile::tempdir()?;
        assert!(matches!(
            regenerate(dir.path()),
            Err(FormalismError::Missing)
        ));
        Ok(())
    }

    #[test]
    fn an_invalid_source_is_an_error_and_writes_nothing() -> Result<(), std::io::Error> {
        let dir = project("{")?;
        assert!(matches!(
            regenerate(dir.path()),
            Err(FormalismError::Source(_))
        ));
        assert!(!dir.path().join(FOLDER).join("E1.md").exists());
        Ok(())
    }

    #[test]
    fn a_source_that_is_not_utf8_is_an_error() -> Result<(), std::io::Error> {
        let dir = project("")?;
        std::fs::write(dir.path().join(SOURCE), [0xff, 0xfe])?;
        assert!(matches!(
            regenerate(dir.path()),
            Err(FormalismError::Source(_))
        ));
        Ok(())
    }

    #[test]
    fn it_writes_what_differs_and_never_deletes() -> Result<(), Box<dyn std::error::Error>> {
        let dir = project(SOURCE_TEXT)?;
        let first = regenerate(dir.path())?;
        assert_eq!(
            first.written,
            ["canon/formalism/E1.md", "canon/formalism/E2.md"]
        );
        assert_eq!(first.unchanged, Vec::<String>::new());
        let stray = dir.path().join(FOLDER).join("E9.md");
        std::fs::write(&stray, "kept")?;
        let e2 = dir.path().join(FOLDER).join("E2.md");
        std::fs::write(&e2, "edited")?;
        let second = regenerate(dir.path())?;
        assert_eq!(second.written, ["canon/formalism/E2.md"]);
        assert_eq!(second.unchanged, ["canon/formalism/E1.md"]);
        assert!(stray.exists(), "a record the source does not list stays");
        let third = regenerate(dir.path())?;
        assert_eq!(third.written, Vec::<String>::new());
        assert_eq!(third.unchanged.len(), 2);
        Ok(())
    }

    /// Whether `result` refused `path` as a link or a path outside the project.
    fn refused(result: &Result<Regenerated, FormalismError>, path: &Path) -> bool {
        matches!(result, Err(FormalismError::Outside { path: refused }) if refused == path)
    }

    /// A folder outside the project that holds `_source.json`, for a linked `canon/formalism`.
    fn outside_folder() -> Result<tempfile::TempDir, std::io::Error> {
        let outside = tempfile::tempdir()?;
        std::fs::write(outside.path().join("_source.json"), SOURCE_TEXT)?;
        Ok(outside)
    }

    #[test]
    fn a_path_that_resolves_outside_the_project_is_refused()
    -> Result<(), Box<dyn std::error::Error>> {
        let (project, other) = (tempfile::tempdir()?, tempfile::tempdir()?);
        let inside = project.path().join("x.md");
        let elsewhere = other.path().join("x.md");
        std::fs::write(&inside, "x")?;
        std::fs::write(&elsewhere, "x")?;
        let root = std::fs::canonicalize(project.path())?;
        assert!(refuse_outside(&root, &inside).is_ok());
        assert!(matches!(
            refuse_outside(&root, &elsewhere),
            Err(FormalismError::Outside { path }) if path == elsewhere
        ));
        assert!(matches!(
            refuse_outside(&root, &project.path().join("gone.md")),
            Err(FormalismError::Io { .. })
        ));
        Ok(())
    }

    #[cfg(unix)]
    #[test]
    fn a_record_that_is_a_link_is_never_written_through() -> Result<(), Box<dyn std::error::Error>>
    {
        let dir = project(SOURCE_TEXT)?;
        let outside = tempfile::tempdir()?;
        let victim = outside.path().join("victim.md");
        std::fs::write(&victim, "kept")?;
        let e2 = dir.path().join(FOLDER).join("E2.md");
        std::os::unix::fs::symlink(&victim, &e2)?;
        let result = regenerate(dir.path());
        assert!(refused(&result, &e2), "{result:?}");
        assert_eq!(std::fs::read_to_string(&victim)?, "kept");
        assert!(
            !dir.path().join(FOLDER).join("E1.md").exists(),
            "a refused record leaves every record unwritten"
        );
        Ok(())
    }

    #[cfg(unix)]
    #[test]
    fn a_folder_that_is_a_link_is_never_written_through() -> Result<(), Box<dyn std::error::Error>>
    {
        // `canon/formalism` is a link.
        let dir = tempfile::tempdir()?;
        let outside = outside_folder()?;
        std::fs::create_dir_all(dir.path().join("canon"))?;
        let folder = dir.path().join("canon").join("formalism");
        std::os::unix::fs::symlink(outside.path(), &folder)?;
        let result = regenerate(dir.path());
        assert!(refused(&result, &folder), "{result:?}");
        assert!(!outside.path().join("E1.md").exists());

        // `canon` is a link.
        let dir = tempfile::tempdir()?;
        let outside = tempfile::tempdir()?;
        std::fs::create_dir_all(outside.path().join("formalism"))?;
        std::fs::write(
            outside.path().join("formalism").join("_source.json"),
            SOURCE_TEXT,
        )?;
        let canon = dir.path().join("canon");
        std::os::unix::fs::symlink(outside.path(), &canon)?;
        let result = regenerate(dir.path());
        assert!(refused(&result, &canon), "{result:?}");
        assert!(!outside.path().join("formalism").join("E1.md").exists());
        Ok(())
    }

    #[cfg(windows)]
    #[test]
    fn a_folder_that_is_a_junction_is_never_written_through()
    -> Result<(), Box<dyn std::error::Error>> {
        // A junction needs no privilege, unlike a symbolic link.
        let dir = tempfile::tempdir()?;
        let outside = outside_folder()?;
        std::fs::create_dir_all(dir.path().join("canon"))?;
        let folder = dir.path().join("canon").join("formalism");
        let made = std::process::Command::new("cmd")
            .args(["/C", "mklink", "/J"])
            .arg(&folder)
            .arg(outside.path())
            .output()?;
        assert!(made.status.success(), "{made:?}");
        let result = regenerate(dir.path());
        assert!(refused(&result, &folder), "{result:?}");
        assert!(!outside.path().join("E1.md").exists());
        Ok(())
    }
}
