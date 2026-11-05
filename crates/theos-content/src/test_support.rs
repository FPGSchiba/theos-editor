//! Helpers shared by unit tests.

use crate::record::Record;

/// Parses `---\n{front}---\n{body}`; panics on error (tests only).
pub(crate) fn record(front: &str, body: &str) -> Record {
    match Record::parse(&format!("---\n{front}---\n{body}")) {
        Ok(record) => record,
        Err(error) => panic!("test record does not parse: {error}"),
    }
}

/// The rule name and `line:column` of the first front-matter error of `front` (tests only).
pub(crate) fn front_error(front: &str) -> Option<(&'static str, String)> {
    record(front, "")
        .front_matter_error()
        .map(|error| (error.rule.as_str(), error.position.to_string()))
}
