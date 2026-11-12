//! Places and floors (spec §5.6): the floor a place stands on, and whether one place lies inside
//! another. Both walk `parent` links through the context, so every step is a stored read.

use std::rc::Rc;

use crate::index::Slot;
use crate::lore::{Context, FactError, RecordFacts};

/// The floor of a place, with the place that gives it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Floor {
    /// The floor number, 1 to 100.
    pub number: u32,
    /// The id of the floor, or of the band that counts as its first floor, that gives the number.
    pub place: String,
}

/// The floor of `place`: a floor's `number`, a band's first floor, or, for a location, the floor
/// of its parent chain. `None` when the chain dangles, links to a wrong kind, meets an invalid
/// value or loops, and for an id that is no place. When the id is duplicated, the first record
/// by path counts.
///
/// # Errors
///
/// Fails when the store fails.
pub fn floor_of(ctx: &mut Context<'_>, place: &str) -> Result<Option<Floor>, FactError> {
    let mut seen: Vec<String> = Vec::new();
    let mut current = first_record(ctx, place)?;
    while let Some(record) = current {
        if seen.contains(&record.path) {
            return Ok(None);
        }
        seen.push(record.path.clone());
        match record.kind.as_deref() {
            Some("floor") => return Ok(number(&record, "number", None)),
            Some("band") => return Ok(number(&record, "floors", Some(0))),
            Some("location") => current = parent_of(ctx, &record)?,
            _ => return Ok(None),
        }
    }
    Ok(None)
}

/// Whether `inner` is `outer`, or `outer` is on the parent chain of `inner`: a location lies
/// inside its floor and that floor's band. A chain that dangles, links to a wrong kind, meets an
/// invalid value or loops ends there.
///
/// # Errors
///
/// Fails when the store fails.
pub fn inside(ctx: &mut Context<'_>, inner: &str, outer: &str) -> Result<bool, FactError> {
    if inner == outer {
        return Ok(true);
    }
    let mut seen: Vec<String> = Vec::new();
    let mut current = first_record(ctx, inner)?;
    while let Some(record) = current {
        if seen.contains(&record.path) {
            return Ok(false);
        }
        seen.push(record.path.clone());
        current = parent_of(ctx, &record)?;
        if current.as_ref().is_some_and(|parent| parent.id == outer) {
            return Ok(true);
        }
    }
    Ok(false)
}

/// The first record by path with `id`.
fn first_record(ctx: &mut Context<'_>, id: &str) -> Result<Option<Rc<RecordFacts>>, FactError> {
    Ok(ctx.records(id)?.into_iter().next())
}

/// The record that the valid `parent` link of a location or floor names, when its kind is one the
/// field allows (a location's parent is a floor or a location; a floor's parent is a band).
fn parent_of(
    ctx: &mut Context<'_>,
    record: &RecordFacts,
) -> Result<Option<Rc<RecordFacts>>, FactError> {
    let allowed: &[&str] = match record.kind.as_deref() {
        Some("location") => &["floor", "location"],
        Some("floor") => &["band"],
        _ => return Ok(None),
    };
    let parent = Slot::Field("parent".to_owned());
    let Some(link) = record.links_in(&parent).next() else {
        return Ok(None);
    };
    Ok(first_record(ctx, &link.target)?.filter(|found| {
        found
            .kind
            .as_deref()
            .is_some_and(|kind| allowed.contains(&kind))
    }))
}

/// The floor a floor's `number` gives, or a band's `floors` item `item`; none when the value is
/// invalid or not a number.
fn number(record: &RecordFacts, field: &str, item: Option<usize>) -> Option<Floor> {
    let value = match item {
        None => record.text(field)?,
        Some(index) => *record
            .items(field)
            .iter()
            .find(|value| value.item == Some(index))?,
    };
    let number = value.text.as_deref()?.parse().ok()?;
    Some(Floor {
        number,
        place: record.id.clone(),
    })
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::test_support::with_context;

    /// A tower with two bands, floors, locations, a dangling parent and a loop. Synthetic.
    pub(crate) const FILES: &[(&str, &str)] = &[
        (
            "world/synthetic-low.md",
            "---\nid: synthetic-low\nkind: band\ntitle: low\nparent: synthetic-tower\nfloors: [3, 5]\n---\n",
        ),
        (
            "world/synthetic-high.md",
            "---\nid: synthetic-high\nkind: band\ntitle: high\nparent: synthetic-tower\nfloors: [20, 20]\n---\n",
        ),
        (
            "world/synthetic-low/f04-synthetic.md",
            "---\nid: f04-synthetic\nkind: floor\ntitle: f4\nparent: synthetic-low\nnumber: 4\n---\n",
        ),
        (
            "world/synthetic-low/f04-synthetic/synthetic-hall.md",
            "---\nid: synthetic-hall\nkind: location\ntitle: hall\nparent: f04-synthetic\n---\n",
        ),
        (
            "world/synthetic-low/f04-synthetic/synthetic-hall/synthetic-nook.md",
            "---\nid: synthetic-nook\nkind: location\ntitle: nook\nparent: synthetic-hall\n---\n",
        ),
        (
            "world/synthetic-low/f04-synthetic/synthetic-lost.md",
            "---\nid: synthetic-lost\nkind: location\ntitle: lost\nparent: synthetic-nowhere\n---\n",
        ),
        (
            "world/synthetic-low/f04-synthetic/synthetic-loop-a.md",
            "---\nid: synthetic-loop-a\nkind: location\ntitle: a\nparent: synthetic-loop-b\n---\n",
        ),
        (
            "world/synthetic-low/f04-synthetic/synthetic-loop-b.md",
            "---\nid: synthetic-loop-b\nkind: location\ntitle: b\nparent: synthetic-loop-a\n---\n",
        ),
        (
            "world/synthetic-low/f04-synthetic/synthetic-wrong.md",
            "---\nid: synthetic-wrong\nkind: location\ntitle: wrong\nparent: synthetic-high\n---\n",
        ),
        (
            "world/synthetic-low/f05-synthetic.md",
            "---\nid: f05-synthetic\nkind: floor\ntitle: f5\nparent: synthetic-low\nnumber: 500\n---\n",
        ),
        (
            "world/synthetic-low/f05-synthetic/synthetic-bad.md",
            "---\nid: synthetic-bad\nkind: location\ntitle: bad\nparent: f05-synthetic\n---\n",
        ),
    ];

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    fn floor(number: u32, place: &str) -> Floor {
        Floor {
            number,
            place: place.to_owned(),
        }
    }

    #[test]
    fn a_band_counts_as_its_first_floor() -> TestResult {
        with_context(FILES, |ctx| {
            assert_eq!(
                floor_of(ctx, "synthetic-low")?,
                Some(floor(3, "synthetic-low"))
            );
            assert_eq!(
                floor_of(ctx, "synthetic-high")?,
                Some(floor(20, "synthetic-high"))
            );
            assert_eq!(
                floor_of(ctx, "f04-synthetic")?,
                Some(floor(4, "f04-synthetic"))
            );
            Ok::<_, FactError>(())
        })??;
        Ok(())
    }

    #[test]
    fn a_location_takes_the_floor_of_its_parent_chain() -> TestResult {
        with_context(FILES, |ctx| {
            assert_eq!(
                floor_of(ctx, "synthetic-hall")?,
                Some(floor(4, "f04-synthetic"))
            );
            assert_eq!(
                floor_of(ctx, "synthetic-nook")?,
                Some(floor(4, "f04-synthetic"))
            );
            Ok::<_, FactError>(())
        })??;
        Ok(())
    }

    #[test]
    fn a_broken_or_looping_chain_has_no_floor() -> TestResult {
        with_context(FILES, |ctx| {
            for id in [
                "synthetic-lost",
                "synthetic-loop-a",
                "synthetic-wrong",
                "synthetic-bad",
                "f05-synthetic",
                "synthetic-tower",
                "nobody",
            ] {
                assert_eq!(floor_of(ctx, id)?, None, "{id}");
            }
            Ok::<_, FactError>(())
        })??;
        Ok(())
    }

    #[test]
    fn inside_follows_the_parent_chain() -> TestResult {
        with_context(FILES, |ctx| {
            assert!(inside(ctx, "synthetic-nook", "synthetic-nook")?);
            assert!(inside(ctx, "synthetic-nook", "synthetic-hall")?);
            assert!(inside(ctx, "synthetic-nook", "f04-synthetic")?);
            assert!(inside(ctx, "synthetic-nook", "synthetic-low")?);
            assert!(inside(ctx, "f04-synthetic", "synthetic-low")?);
            assert!(!inside(ctx, "synthetic-low", "synthetic-nook")?);
            assert!(!inside(ctx, "synthetic-nook", "synthetic-high")?);
            assert!(!inside(ctx, "synthetic-lost", "synthetic-nowhere")?);
            assert!(!inside(ctx, "synthetic-loop-a", "synthetic-low")?);
            assert!(!inside(ctx, "synthetic-wrong", "synthetic-high")?);
            Ok::<_, FactError>(())
        })??;
        Ok(())
    }
}
