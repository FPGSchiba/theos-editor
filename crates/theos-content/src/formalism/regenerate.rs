//! Writing the equation records of a project from its source.

use std::path::{Path, PathBuf};

use super::{FOLDER, SOURCE, SourceError, parse, render};

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
    /// The project has no `canon/formalism/_source.json`.
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
}

/// Writes each equation record of `<root>/canon/formalism/_source.json` whose bytes differ from
/// its rendering (LF). It writes nothing else and never deletes: a record
/// the source does not list stays, for `formalism/stray` to report.
///
/// # Errors
///
/// [`FormalismError`] when there is no source, it is not valid, or a file cannot be read or
/// written.
pub fn regenerate(root: &Path) -> Result<Regenerated, FormalismError> {
    let source_path = root.join(SOURCE);
    let bytes = match std::fs::read(&source_path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Err(FormalismError::Missing);
        }
        Err(error) => return Err(io(source_path, error)),
    };
    let text = String::from_utf8(bytes)
        .map_err(|_| SourceError::format("the file is not UTF-8".to_owned()))?;
    let files = render(&parse(&text)?)?;
    let folder = root.join(FOLDER);
    std::fs::create_dir_all(&folder).map_err(|error| io(folder, error))?;
    let mut done = Regenerated::default();
    for (path, text) in files {
        let file = root.join(&path);
        if std::fs::read(&file).is_ok_and(|held| held == text.as_bytes()) {
            done.unchanged.push(path);
        } else {
            std::fs::write(&file, text).map_err(|error| io(file, error))?;
            done.written.push(path);
        }
    }
    Ok(done)
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
}
