//! The value checks of concepts, maps and their versions (spec §5.10). Both parts register the
//! same rule ids, each for its own kinds, so a rule is built from the kinds it is on.
//!
//! A version's parent is named by the field that is the parent kind's name: `concept` for a
//! `concept-version`, `map` for a `map-version`.

mod addresses;
mod from;
mod picked;

pub(crate) use addresses::addresses;
pub(crate) use from::from;
pub(crate) use picked::picked;

use std::rc::Rc;

use crate::index::Slot;
use crate::lore::{Context, FactError, RecordFacts};

/// The id that the parent field of the version `record` names; none when the field has no valid
/// link.
fn parent_id(record: &RecordFacts, parent_kind: &str) -> Option<String> {
    let slot = Slot::Field(parent_kind.to_owned());
    record
        .links_in(&slot)
        .next()
        .map(|link| link.target.clone())
}

/// The first record by path with `id` and the kind `kind`.
fn record_of_kind(
    ctx: &mut Context<'_>,
    id: &str,
    kind: &str,
) -> Result<Option<Rc<RecordFacts>>, FactError> {
    Ok(ctx
        .records(id)?
        .into_iter()
        .find(|found| found.kind.as_deref() == Some(kind)))
}
