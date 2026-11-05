//! `record/utf8`: the bytes are not UTF-8.

use crate::finding::{Finding, Severity};
use crate::record::ReadError;
use crate::text::{Span, normalize};

/// The rule id.
pub(super) const RULE: &str = "record/utf8";

/// The finding for a failed decode: an empty span at the first bad byte, counted in the LF text
/// of the valid prefix. `None` when `error` is not a UTF-8 error.
pub(super) fn finding(path: &str, id: &str, bytes: &[u8], error: &ReadError) -> Option<Finding> {
    let ReadError::Utf8 { position } = error else {
        return None;
    };
    let valid = std::str::from_utf8(bytes)
        .err()
        .map_or(0, |e| e.valid_up_to());
    let prefix = bytes
        .get(..valid)
        .and_then(|valid| std::str::from_utf8(valid).ok())
        .unwrap_or_default();
    let offset = normalize(prefix).0.len();
    Some(Finding {
        rule: RULE.to_owned(),
        severity: Severity::Error,
        path: path.to_owned(),
        id: Some(id.to_owned()),
        span: Some(Span::new(offset, offset)),
        position: Some(*position),
        message: "the file is not valid UTF-8".to_owned(),
    })
}
