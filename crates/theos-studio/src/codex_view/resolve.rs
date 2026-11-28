//! Turns the id a link names into the record it names, with P48's marks, once per view.

use std::collections::HashMap;

use serde::Serialize;
use theos_content::RecordSummary;

use crate::views::codex::RecordRef;
use crate::{Marks, StudioError, Workspace};

/// A link as the Codex draws it: a chip with the record it names, or a red one when it dangles.
#[derive(Clone, Debug, Serialize)]
pub struct ViewLink {
    /// The id the link names.
    pub id: String,
    /// The text after `#`, if any.
    pub fragment: Option<String>,
    /// The text after `|`, if any.
    pub shown: Option<String>,
    /// The record the id names; `None` when no record has it.
    pub target: Option<RecordRef>,
}

/// Every record by id, with the marks, read once from the index.
pub struct Resolver {
    marks: Marks,
    by_id: HashMap<String, RecordSummary>,
}

impl Resolver {
    /// Reads the records and marks of `ws`.
    ///
    /// # Errors
    /// [`StudioError::Index`] when the index fails.
    pub fn load(ws: &Workspace) -> Result<Resolver, StudioError> {
        let mut by_id = HashMap::new();
        // Ordered by path, so the first record of a duplicated id stays, as in `codex::read`.
        for summary in ws.index().records()? {
            by_id.entry(summary.id.clone()).or_insert(summary);
        }
        Ok(Resolver {
            marks: Marks::load(ws.index())?,
            by_id,
        })
    }

    /// The marks of every record.
    #[must_use]
    pub fn marks(&self) -> &Marks {
        &self.marks
    }

    /// The record `id` names, with the marks of every record that has the id.
    #[must_use]
    pub fn record(&self, id: &str) -> Option<RecordRef> {
        self.by_id.get(id).map(|summary| RecordRef {
            mark: self.marks.of_id(id),
            ..RecordRef::from_summary(summary, &self.marks)
        })
    }

    /// The record `id` names, or the bare id when none has it.
    #[must_use]
    pub fn record_or_missing(&self, id: &str) -> RecordRef {
        self.record(id).unwrap_or_else(|| RecordRef::missing(id))
    }

    /// The chip of a link to `id`.
    #[must_use]
    pub fn link(&self, id: &str, fragment: Option<String>, shown: Option<String>) -> ViewLink {
        ViewLink {
            id: id.to_owned(),
            fragment,
            shown,
            target: self.record(id),
        }
    }

    /// The chip of a body wikilink.
    #[must_use]
    pub fn body_link(&self, link: &theos_content::Link) -> ViewLink {
        self.link(&link.target, link.fragment.clone(), link.shown.clone())
    }
}
