//! Turns the id a link names into the record it names, with P48's marks, once per view. It
//! reads through P48's lookup (`codex::Lookup`), which `codex::read` shares, so a page loads the
//! records and marks once.

use serde::Serialize;

use crate::codex::Lookup;
use crate::views::codex::RecordRef;
use crate::{StudioError, Workspace};

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
    lookup: Lookup,
}

impl Resolver {
    /// Reads the records and marks of `ws`.
    ///
    /// # Errors
    /// [`StudioError::Index`] when the index fails.
    pub fn load(ws: &Workspace) -> Result<Resolver, StudioError> {
        Ok(Resolver {
            lookup: Lookup::load(ws)?,
        })
    }

    /// The lookup the resolver reads, for `codex::read` to share.
    pub(crate) fn lookup(&self) -> &Lookup {
        &self.lookup
    }

    /// The record `id` names (the first by path when several have it), with the marks of every
    /// record that has the id.
    #[must_use]
    pub fn record(&self, id: &str) -> Option<RecordRef> {
        let target = self.lookup.target(id);
        target.exists.then_some(target.record)
    }

    /// The record `id` names, or the bare id when none has it.
    #[must_use]
    pub fn record_or_missing(&self, id: &str) -> RecordRef {
        self.lookup.target(id).record
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
