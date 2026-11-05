//! Helpers shared by unit tests.

use crate::record::Record;

/// Parses `---\n{front}---\n{body}`; panics on error (tests only).
pub(crate) fn record(front: &str, body: &str) -> Record {
    match Record::parse(&format!("---\n{front}---\n{body}")) {
        Ok(record) => record,
        Err(error) => panic!("test record does not parse: {error}"),
    }
}
