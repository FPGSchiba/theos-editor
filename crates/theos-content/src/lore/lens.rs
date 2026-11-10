//! The effective lens as lore rules read it (spec §5.5): the one chain of
//! [`crate::index::lens`], fed by recorded reads of the context instead of SQL.

use std::collections::HashMap;
use std::rc::Rc;

use crate::index::Slot;
use crate::index::lens::{
    Layer, LensData, LensSource, Node, Shape, chain, holder_kinds, merge, written,
};
use crate::lore::{Context, FactError, RecordFacts};

/// The ids of the `rejects` entry of the effective lens of `holder`: the nearest layer of its
/// chain that sets the entry wins, and an invalid entry does not count. `None` when `holder` is no
/// id of a record whose kind has a lens; the empty list when no layer sets `rejects`.
///
/// # Errors
///
/// Fails when the store fails.
pub fn rejects(ctx: &mut Context<'_>, holder: &str) -> Result<Option<Vec<String>>, FactError> {
    let registry = ctx.registry();
    let holders = holder_kinds(registry);
    let mut source = Recorded {
        ctx,
        seen: HashMap::new(),
    };
    let Some(first) = source.holder(holder, &holders)? else {
        return Ok(None);
    };
    let layers = chain(&mut source, registry, first, &holders)?;
    let mut writes = Vec::new();
    for layer in &layers {
        writes.push(written(&mut source, layer)?);
    }
    Ok(Some(
        match merge("rejects", &writes).map(|value| value.value) {
            Some(LensData::Links(ids)) => ids,
            _ => Vec::new(),
        },
    ))
}

/// The lens rows of the records the chain visits, read through the context. The trait asks by
/// path where the context answers by id, so the records the walk found are kept by path.
struct Recorded<'c, 'a> {
    ctx: &'c mut Context<'a>,
    seen: HashMap<String, Rc<RecordFacts>>,
}

impl LensSource for Recorded<'_, '_> {
    type Error = FactError;

    fn holder(&mut self, id: &str, kinds: &[String]) -> Result<Option<Layer>, FactError> {
        for record in self.ctx.records(id)? {
            if let Some(kind) = record.kind.clone().filter(|kind| kinds.contains(kind)) {
                let layer = Layer {
                    id: id.to_owned(),
                    kind,
                    path: record.path.clone(),
                };
                self.seen.insert(record.path.clone(), record);
                return Ok(Some(layer));
            }
        }
        Ok(None)
    }

    fn parent(&mut self, path: &str) -> Result<Option<String>, FactError> {
        let parent = Slot::Field("parent".to_owned());
        Ok(self
            .seen
            .get(path)
            .and_then(|record| record.links_in(&parent).next())
            .map(|link| link.target.clone()))
    }

    fn lens_nodes(&mut self, path: &str) -> Result<Vec<Node>, FactError> {
        let Some(record) = self.seen.get(path) else {
            return Ok(Vec::new());
        };
        let has_entries = record
            .values
            .iter()
            .any(|value| value.field == "lens" && value.entry.is_some());
        Ok(record
            .values
            .iter()
            .filter(|value| value.field == "lens")
            .map(|value| Node {
                entry: value.entry.clone(),
                item: value.item.is_some(),
                // The facts do not keep the shape: a value without text is a list or a map. The
                // lens itself is a map when it has entries; any other value without text counts
                // as a list, the only non-scalar shape a valid entry has. An empty map and an
                // empty list set nothing either way.
                shape: if value.text.is_some() {
                    Shape::Scalar
                } else if value.entry.is_none() && value.item.is_none() && has_entries {
                    Shape::Map
                } else {
                    Shape::List
                },
                text: value.text.clone(),
                span: value.span,
                position: value.position,
                valid: value.valid,
            })
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::indexed;

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    fn holder(kind: &str, id: &str, parent: Option<&str>, lens: &str) -> (String, String) {
        let parent = parent.map_or_else(String::new, |parent| {
            format!(
                "parent: {parent}
"
            )
        });
        (
            format!("holders/{id}.md"),
            format!(
                "---
id: {id}
kind: {kind}
title: t
{parent}{lens}---
"
            ),
        )
    }

    #[test]
    fn rejects_equals_the_effective_lens() -> TestResult {
        let top_lens = "lens:
  rejects: [synthetic-b1, synthetic-b2]
  stance: calm
";
        let owned = [
            holder("people", "synthetic-top", None, top_lens),
            holder(
                "faction",
                "synthetic-mid",
                Some("synthetic-top"),
                "lens:
  rejects: [synthetic-b3]
",
            ),
            holder(
                "npc",
                "synthetic-leaf",
                Some("synthetic-mid"),
                "lens:
  stance: x
",
            ),
            holder(
                "npc",
                "synthetic-inherit",
                Some("synthetic-top"),
                "lens:
  stance: y
",
            ),
            holder(
                "faction",
                "synthetic-empty",
                Some("synthetic-top"),
                "lens:
  rejects: []
",
            ),
            holder(
                "faction",
                "synthetic-bad",
                Some("synthetic-top"),
                "lens:
  rejects: nonsense
",
            ),
            holder(
                "faction",
                "synthetic-notmap",
                Some("synthetic-top"),
                "lens: [a, b]
",
            ),
            holder(
                "faction",
                "synthetic-lost",
                Some("synthetic-nowhere"),
                "lens:
  tier: T2
",
            ),
            holder("people", "synthetic-none", None, ""),
        ];
        let files: Vec<(&str, &str)> = owned
            .iter()
            .map(|(path, text)| (path.as_str(), text.as_str()))
            .collect();
        let (_dir, index) = indexed(&files)?;
        let mut ctx = index.lore_context();
        let mut with_rejects = 0;
        for (path, _) in &owned {
            let id = path.trim_start_matches("holders/").trim_end_matches(".md");
            let Some(lens) = index.lens(id)? else {
                return Err(format!("no lens for {id}").into());
            };
            let expected = match lens.fields.iter().find(|f| f.field == "rejects") {
                Some(field) => match &field.value {
                    LensData::Links(ids) => ids.clone(),
                    LensData::Text(_) => return Err("rejects is text".into()),
                },
                None => Vec::new(),
            };
            with_rejects += usize::from(!expected.is_empty());
            assert_eq!(rejects(&mut ctx, id)?, Some(expected), "{id}");
        }
        assert!(with_rejects >= 3, "the project must exercise inheritance");
        assert_eq!(rejects(&mut ctx, "nobody")?, None);
        Ok(())
    }
}
