//! The `links` participant: the records to edit are the renamed records, for their `id` field,
//! and every record the index knows to link to an old id.

use super::participant::{Participant, RenameSet, record};
use crate::command::{CommandError, PlanContext};

/// Adds each renamed record and the source of every backlink of an old id to the sources.
pub(crate) struct Links;

impl Participant for Links {
    fn extend(&self, ctx: &PlanContext<'_>, set: &mut RenameSet) -> Result<(), CommandError> {
        for old in set.ids.keys() {
            set.sources.insert(record(ctx, old)?.path);
            for link in ctx.index().backlinks(old)? {
                set.sources.insert(link.source_path);
            }
        }
        Ok(())
    }
}
