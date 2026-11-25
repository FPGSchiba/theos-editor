//! The `ids` participant: each renamed record's file takes its new id as its name, in its
//! folder, or in the folder it moves to with a renamed parent's child folder.

use super::participant::{Participant, RenameSet, folder_of, join, record};
use crate::command::{CommandError, PlanContext};

/// Moves each renamed record to `<folder>/<new id>.md`; runs after [`Children`].
///
/// [`Children`]: super::children::Children
pub(crate) struct Ids;

impl Participant for Ids {
    fn extend(&self, ctx: &PlanContext<'_>, set: &mut RenameSet) -> Result<(), CommandError> {
        for (old, new) in &set.ids {
            let path = record(ctx, old)?.path;
            let at = set.moves.get(&path).unwrap_or(&path);
            let target = join(folder_of(at), &format!("{new}.md"));
            set.moves.insert(path, target);
        }
        Ok(())
    }
}
