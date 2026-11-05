//! Reading records from bytes and files.

use std::path::Path;

use super::{ReadError, Record};
use crate::text::{Position, TextIndex, normalize};

/// Checks that `bytes` are UTF-8.
///
/// # Errors
///
/// [`ReadError::Utf8`] with the position of the first invalid byte, counted in the normalised
/// valid prefix.
pub fn decode(bytes: &[u8]) -> Result<&str, ReadError> {
    std::str::from_utf8(bytes).map_err(|error| {
        let prefix = bytes
            .get(..error.valid_up_to())
            .and_then(|valid| std::str::from_utf8(valid).ok())
            .unwrap_or_default();
        let index = TextIndex::new(normalize(prefix).0.into_owned());
        let position = index
            .position(index.text().len())
            .unwrap_or(Position { line: 1, column: 1 });
        ReadError::Utf8 { position }
    })
}

/// Reads and parses the record at `path`.
///
/// # Errors
///
/// [`ReadError::Io`], [`ReadError::Utf8`] or [`ReadError::Record`].
pub fn read_record(path: &Path) -> Result<Record, ReadError> {
    let bytes = std::fs::read(path).map_err(|source| ReadError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    Ok(Record::parse(decode(&bytes)?)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invalid_utf8_reports_its_position() {
        for bytes in [&b"---\nid: \xff\n"[..], &b"---\r\nid: \xff"[..]] {
            match decode(bytes) {
                Err(ReadError::Utf8 { position }) => {
                    assert_eq!(position, Position { line: 2, column: 5 });
                }
                other => panic!("expected Utf8, got {other:?}"),
            }
        }
        assert_eq!(decode(b"---\n").ok(), Some("---\n"));
    }

    #[test]
    fn read_record_reads_a_file() -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempfile::tempdir()?;
        let path = dir.path().join("a.md");
        std::fs::write(&path, "---\nid: a\n---\nbody\n")?;
        assert_eq!(read_record(&path)?.text(), "---\nid: a\n---\nbody\n");
        assert!(matches!(
            read_record(&dir.path().join("missing.md")),
            Err(ReadError::Io { .. })
        ));
        Ok(())
    }
}
