//! "Used by": the backlinks of a record grouped by source, each with the relation its slot names
//! (P11 §5.8). Nothing is stored; the rows are the index's backlinks at the time of the view.

use serde::Serialize;

use crate::views::codex::{LinkView, RecordRef};

/// One source that links a record, with each of its relations once.
#[derive(Clone, Debug, Serialize)]
pub struct UsedBy {
    /// The record that holds the links.
    pub source: RecordRef,
    /// The relations, in the order the links come.
    pub relations: Vec<String>,
}

/// The relation a backlink's slot (P03's `Slot::as_db` text) names: `field:<key>` gives the key
/// with `_` as a space, and `parent` as "child of"; `beat:*` gives "in a beat"; `voice:*`
/// "voice"; `lens:*` "lens"; `note:*` "note"; `mention` "mentions". Any other slot is itself.
#[must_use]
pub fn relation(slot: &str) -> String {
    if slot == "mention" {
        return "mentions".to_owned();
    }
    match slot.split_once(':') {
        Some(("field", "parent")) => "child of".to_owned(),
        Some(("field", key)) => key.replace('_', " "),
        Some(("beat", _)) => "in a beat".to_owned(),
        Some(("voice", _)) => "voice".to_owned(),
        Some(("lens", _)) => "lens".to_owned(),
        Some(("note", _)) => "note".to_owned(),
        _ => slot.to_owned(),
    }
}

/// The backlinks grouped by source path, in the order the sources first come.
#[must_use]
pub fn used_by(backlinks: &[LinkView]) -> Vec<UsedBy> {
    let mut groups: Vec<UsedBy> = Vec::new();
    for link in backlinks {
        let relation = relation(&link.slot);
        let same = |used: &&mut UsedBy| {
            used.source.path == link.source.path && used.source.id == link.source.id
        };
        match groups.iter_mut().find(same) {
            Some(group) => {
                if !group.relations.contains(&relation) {
                    group.relations.push(relation);
                }
            }
            None => groups.push(UsedBy {
                source: link.source.clone(),
                relations: vec![relation],
            }),
        }
    }
    groups
}
