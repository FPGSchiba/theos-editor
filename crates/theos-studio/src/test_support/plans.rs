//! Plans built by hand, for the write and the pipeline.

use crate::{FileChange, Plan};

/// A change of `path` from the text `before` to the text `after`; `None` for a file that is
/// absent.
pub(crate) fn change(path: &str, before: Option<&str>, after: Option<&str>) -> FileChange {
    FileChange {
        path: path.to_owned(),
        before: before.map(|text| text.as_bytes().to_vec()),
        after: after.map(|text| text.as_bytes().to_vec()),
    }
}

/// A plan of `changes` that moves no file.
pub(crate) fn plan(changes: Vec<FileChange>) -> Plan {
    Plan {
        changes,
        ..Plan::default()
    }
}
