//! The lookup tables of one Codex call: records by path and id, and the marks.

use std::collections::HashMap;

use theos_content::{Hit, Layer, LinkRef, RecordSummary};

use crate::views::codex::{LinkView, RecordRef, TargetRef};
use crate::{Marks, StudioError, Workspace};

/// Everything one call looks records up in, built once. `read` and `graph` build link targets
/// through it, so both carry the same marks.
pub(super) struct Lookup {
    pub(super) marks: Marks,
    pub(super) by_path: HashMap<String, RecordSummary>,
    pub(super) by_id: HashMap<String, RecordSummary>,
}

impl Lookup {
    pub(super) fn load(ws: &Workspace) -> Result<Lookup, StudioError> {
        let mut by_path = HashMap::new();
        let mut by_id: HashMap<String, RecordSummary> = HashMap::new();
        // Ordered by path, so the first record of a duplicated id stays.
        for summary in ws.index().records()? {
            by_id
                .entry(summary.id.clone())
                .or_insert_with(|| summary.clone());
            by_path.insert(summary.path.clone(), summary);
        }
        Ok(Lookup {
            marks: Marks::load(ws.index())?,
            by_path,
            by_id,
        })
    }

    pub(super) fn of_layer(&self, layer: &Layer) -> RecordRef {
        self.of_path(&layer.path, &layer.id, Some(&layer.kind))
    }

    /// The record at `path`; a path the index lacks falls back to the words of the caller.
    pub(super) fn of_path(&self, path: &str, id: &str, kind: Option<&str>) -> RecordRef {
        self.resolve(path, id, kind, None)
    }

    /// The record of a search hit; a path the index lacks falls back to the hit's own words.
    pub(super) fn of_hit(&self, hit: &Hit) -> RecordRef {
        self.resolve(
            &hit.path,
            &hit.id,
            hit.kind.as_deref(),
            hit.title.as_deref(),
        )
    }

    /// The one fallback for a path the index lacks: id, kind and title as the caller knew
    /// them, the marks of the path, and no status.
    fn resolve(&self, path: &str, id: &str, kind: Option<&str>, title: Option<&str>) -> RecordRef {
        if let Some(summary) = self.by_path.get(path) {
            return RecordRef::from_summary(summary, &self.marks);
        }
        tracing::warn!(path, id, "the index lacks a path that it named");
        RecordRef {
            path: Some(path.to_owned()),
            kind: kind.map(str::to_owned),
            title: title.map(str::to_owned),
            mark: self.marks.of_path(path),
            ..RecordRef::missing(id)
        }
    }

    /// The record an id names, with the marks of every record that has the id.
    pub(super) fn target(&self, id: &str) -> TargetRef {
        match self.by_id.get(id) {
            Some(summary) => TargetRef {
                record: RecordRef {
                    mark: self.marks.of_id(id),
                    ..RecordRef::from_summary(summary, &self.marks)
                },
                exists: true,
            },
            None => TargetRef {
                record: RecordRef::missing(id),
                exists: false,
            },
        }
    }

    pub(super) fn link(&self, link: &LinkRef) -> LinkView {
        LinkView {
            slot: link.slot.as_db(),
            fragment: link.fragment.clone(),
            line: saturate(link.position.line),
            column: saturate(link.position.column),
            source: self.of_path(&link.source_path, &link.source_id, None),
            target: self.target(&link.target),
        }
    }
}

pub(super) fn saturate(value: usize) -> u32 {
    u32::try_from(value).unwrap_or(u32::MAX)
}
