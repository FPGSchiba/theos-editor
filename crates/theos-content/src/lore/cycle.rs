//! Cycles along one link field (spec §5.10): whether a record's link leads back to the record.

use std::collections::HashSet;

use crate::index::Slot;
use crate::lore::{Context, FactError, RecordFacts};

/// The index, among the links of `start` in the front-matter field `field`, of the first one that
/// leads into a cycle back to `start`; `None` when none does. Only valid links of `field` are
/// followed, through every record with the target id, and a target without records ends a path.
/// A cycle is found from each of its members and from no record that only leads into it.
///
/// # Errors
///
/// Fails when the store fails.
pub fn cycle_through(
    ctx: &mut Context<'_>,
    start: &RecordFacts,
    field: &str,
) -> Result<Option<usize>, FactError> {
    let slot = Slot::Field(field.to_owned());
    for (index, link) in start.links_in(&slot).enumerate() {
        if reaches(ctx, &link.target, &start.id, &slot)? {
            return Ok(Some(index));
        }
    }
    Ok(None)
}

/// Whether following `slot` links from the id `from` arrives at the id `goal`.
fn reaches(ctx: &mut Context<'_>, from: &str, goal: &str, slot: &Slot) -> Result<bool, FactError> {
    let mut seen: HashSet<String> = HashSet::new();
    let mut stack = vec![from.to_owned()];
    while let Some(id) = stack.pop() {
        if id == goal {
            return Ok(true);
        }
        if !seen.insert(id.clone()) {
            continue;
        }
        for record in ctx.records(&id)? {
            stack.extend(record.links_in(slot).map(|link| link.target.clone()));
        }
    }
    Ok(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::with_context;

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    fn location(id: &str, parent: &str) -> (String, String) {
        (
            format!("world/{id}.md"),
            format!("---\nid: {id}\nkind: location\ntitle: t\nparent: {parent}\n---\n"),
        )
    }

    #[test]
    fn a_cycle_is_found_from_each_member_only() -> TestResult {
        // a -> b -> c -> a is a cycle; d leads into it; e ends at a missing record; f loops on
        // itself.
        let owned = [
            location("synthetic-a", "synthetic-b"),
            location("synthetic-b", "synthetic-c"),
            location("synthetic-c", "synthetic-a"),
            location("synthetic-d", "synthetic-a"),
            location("synthetic-e", "synthetic-missing"),
            location("synthetic-f", "synthetic-f"),
        ];
        let files: Vec<(&str, &str)> = owned
            .iter()
            .map(|(path, text)| (path.as_str(), text.as_str()))
            .collect();
        with_context(&files, |ctx| {
            for (id, expected) in [
                ("synthetic-a", Some(0)),
                ("synthetic-b", Some(0)),
                ("synthetic-c", Some(0)),
                ("synthetic-d", None),
                ("synthetic-e", None),
                ("synthetic-f", Some(0)),
            ] {
                let found = ctx.records(id)?;
                let record = found.first().ok_or("record missing")?;
                assert_eq!(cycle_through(ctx, record, "parent")?, expected, "{id}");
                assert_eq!(cycle_through(ctx, record, "other")?, None, "{id}");
            }
            Ok::<_, Box<dyn std::error::Error>>(())
        })??;
        Ok(())
    }
}
