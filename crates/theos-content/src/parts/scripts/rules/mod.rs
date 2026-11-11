//! The lore rules of the scripts part: what a beat's attributes say against places, lenses and
//! clues (spec §5.9). A beat without `at::` or `when::` skips the checks that need it, and beats
//! that share a number are not judged (see [`single_beats`]).

mod beat_at;
mod reveal_gate;
mod reveal_where;
mod says_rejected;

use std::rc::Rc;

use crate::index::Slot;
use crate::lore::{Context, FactBeat, FactError, FactLink, RecordFacts};

pub(super) use beat_at::BEAT_AT;
pub(super) use reveal_gate::REVEAL_GATE;
pub(super) use reveal_where::REVEAL_WHERE;
pub(super) use says_rejected::SAYS_REJECTED;

/// The beats of `script` whose number no other beat has, in order. A link knows its beat only
/// by number, so the attributes of beats that share one cannot be told apart; such beats are
/// left to their `body/beat-number` finding (one fault, one finding).
fn single_beats(script: &RecordFacts) -> impl Iterator<Item = &FactBeat> {
    script.beats.iter().filter(|beat| {
        script
            .beats
            .iter()
            .filter(|other| other.number == beat.number)
            .count()
            == 1
    })
}

/// The links of the beat attribute `key` in beat `number` of `script`, in file order.
fn beat_links<'a>(
    script: &'a RecordFacts,
    number: u32,
    key: &'a str,
) -> impl Iterator<Item = &'a FactLink> {
    script.links.iter().filter(move |link| {
        link.beat == Some(number) && matches!(&link.slot, Slot::Beat(k) if k == key)
    })
}

/// The first `at::` target of beat `number` that names a place; a dangling or wrong-kind link
/// has a finding of its own and is skipped.
fn beat_at_place(
    ctx: &mut Context<'_>,
    script: &RecordFacts,
    number: u32,
) -> Result<Option<String>, FactError> {
    for link in beat_links(script, number, "at") {
        if is_place(ctx, &link.target)? {
            return Ok(Some(link.target.clone()));
        }
    }
    Ok(None)
}

/// The targets of the links of `record` in the field `field` that name a place.
fn places_in(
    ctx: &mut Context<'_>,
    record: &RecordFacts,
    field: &str,
) -> Result<Vec<String>, FactError> {
    let slot = Slot::Field(field.to_owned());
    let mut places = Vec::new();
    for link in record.links_in(&slot) {
        if is_place(ctx, &link.target)? {
            places.push(link.target.clone());
        }
    }
    Ok(places)
}

/// Whether `place` lies inside one of `wanted`.
fn inside_any(ctx: &mut Context<'_>, place: &str, wanted: &[String]) -> Result<bool, FactError> {
    for outer in wanted {
        if crate::lore::places::inside(ctx, place, outer)? {
            return Ok(true);
        }
    }
    Ok(false)
}

/// Whether some record with `id` is a band, a floor or a location.
fn is_place(ctx: &mut Context<'_>, id: &str) -> Result<bool, FactError> {
    Ok(ctx
        .records(id)?
        .iter()
        .any(|record| matches!(record.kind.as_deref(), Some("band" | "floor" | "location"))))
}

/// The records of kind `clue` with `id`.
fn clue_records(ctx: &mut Context<'_>, id: &str) -> Result<Vec<Rc<RecordFacts>>, FactError> {
    Ok(ctx
        .records(id)?
        .into_iter()
        .filter(|record| record.kind.as_deref() == Some("clue"))
        .collect())
}
