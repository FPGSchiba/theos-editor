//! The pictures of a record: the concepts whose `for` names it, each with its picked version
//! (P11 §5.8). The images themselves arrive with P16.

use serde::Serialize;

use crate::views::codex::{LinkView, RecordRef};
use crate::{StudioError, Workspace};

/// The slot of a concept's `for` link.
const FOR: &str = "field:for";

/// The slot of a concept's `picked` link.
const PICKED: &str = "field:picked";

/// One concept of a record.
#[derive(Clone, Debug, Serialize)]
pub struct PictureRow {
    /// The concept.
    pub concept: RecordRef,
    /// The id of its first picked version; `None` when nothing is picked.
    pub picked: Option<String>,
}

/// The concepts among `backlinks` that name the record in `for`, ordered by id.
///
/// # Errors
/// [`StudioError::Index`] when the index fails.
pub fn rows(ws: &Workspace, backlinks: &[LinkView]) -> Result<Vec<PictureRow>, StudioError> {
    let mut concepts: Vec<&RecordRef> = Vec::new();
    for link in backlinks {
        if link.slot == FOR
            && link.source.kind.as_deref() == Some("concept")
            && !concepts.iter().any(|c| c.path == link.source.path)
        {
            concepts.push(&link.source);
        }
    }
    concepts.sort_by(|a, b| a.id.cmp(&b.id).then_with(|| a.path.cmp(&b.path)));
    concepts
        .into_iter()
        .map(|concept| {
            let picked = match concept.path.as_deref() {
                Some(path) => ws
                    .index()
                    .links_from(path)?
                    .into_iter()
                    .find(|link| link.slot.as_db() == PICKED)
                    .map(|link| link.target),
                None => None,
            };
            Ok(PictureRow {
                concept: concept.clone(),
                picked,
            })
        })
        .collect()
}
