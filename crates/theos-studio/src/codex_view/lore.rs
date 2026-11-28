//! "Lore this record carries": the truths, beliefs and clues that link to a record, each with
//! the truth or topic it touches and its tier (P11 §5.8).

use std::collections::HashMap;

use serde::Serialize;
use theos_content::FieldType;
use theos_content::kinds::TIERS;

use super::resolve::Resolver;
use crate::views::codex::{LinkView, RecordRef};
use crate::{StudioError, Workspace};

/// The kinds whose records fill the lore table.
const LORE_KINDS: &[&str] = &["truth", "belief", "clue"];

/// The fields through which a lore record touches another, with the verb each shows.
const VERBS: &[(&str, &str)] = &[
    ("field:reveals", "hints"),
    ("field:distorts", "distorts"),
    ("field:topic", "about"),
];

/// One row of the lore table.
#[derive(Clone, Debug, Serialize)]
pub struct LoreRow {
    /// The truth, belief or clue.
    pub record: RecordRef,
    /// `hints`, `distorts` or `about`; `None` when the record has no such field.
    pub verb: Option<String>,
    /// The record the verb names, with its marks (`never` among them).
    pub touches: Option<RecordRef>,
    /// The value of the record's tier field (a clue's `gate`), if it has one.
    pub tier: Option<String>,
}

/// The rows for the record at `path`, from its `backlinks`, one per lore record, ordered by id.
///
/// # Errors
/// [`StudioError::Index`] when the index fails.
pub fn rows(
    ws: &Workspace,
    resolver: &Resolver,
    path: &str,
    backlinks: &[LinkView],
) -> Result<Vec<LoreRow>, StudioError> {
    let mut sources: Vec<&RecordRef> = Vec::new();
    for link in backlinks {
        let source = &link.source;
        let lore = source
            .kind
            .as_deref()
            .is_some_and(|kind| LORE_KINDS.contains(&kind));
        if lore
            && source.path.as_deref() != Some(path)
            && !sources.iter().any(|s| s.path == source.path)
        {
            sources.push(source);
        }
    }
    sources.sort_by(|a, b| a.id.cmp(&b.id).then_with(|| a.path.cmp(&b.path)));
    let mut tiers = TierTexts::default();
    sources
        .into_iter()
        .map(|source| row(ws, resolver, &mut tiers, source))
        .collect()
}

fn row(
    ws: &Workspace,
    resolver: &Resolver,
    tiers: &mut TierTexts,
    source: &RecordRef,
) -> Result<LoreRow, StudioError> {
    let Some(path) = source.path.as_deref() else {
        return Ok(LoreRow {
            record: source.clone(),
            verb: None,
            touches: None,
            tier: None,
        });
    };
    let touch = ws.index().links_from(path)?.into_iter().find_map(|link| {
        let slot = link.slot.as_db();
        VERBS
            .iter()
            .find(|(field, _)| *field == slot)
            .map(|(_, verb)| (*verb, link.target))
    });
    let (verb, touches) = match touch {
        Some((verb, target)) => (
            Some(verb.to_owned()),
            Some(resolver.record_or_missing(&target)),
        ),
        None => (None, None),
    };
    Ok(LoreRow {
        record: source.clone(),
        verb,
        touches,
        tier: tiers.of(ws, source.kind.as_deref(), path)?,
    })
}

/// The texts of the tier fields, read from the index once per field key.
#[derive(Default)]
struct TierTexts {
    by_key: HashMap<&'static str, HashMap<String, String>>,
}

impl TierTexts {
    /// The value of the first tier field of `kind` that the record at `path` sets.
    fn of(
        &mut self,
        ws: &Workspace,
        kind: Option<&str>,
        path: &str,
    ) -> Result<Option<String>, StudioError> {
        let Some(spec) = kind.and_then(|kind| ws.registry().kind(kind)) else {
            return Ok(None);
        };
        let keys = spec
            .fields
            .iter()
            .filter(|field| matches!(field.ty, FieldType::Enum(values) if values == TIERS))
            .map(|field| field.key);
        for key in keys {
            if !self.by_key.contains_key(key) {
                let texts = ws
                    .index()
                    .field_texts(key)?
                    .into_iter()
                    .map(|row| (row.path, row.text))
                    .collect();
                self.by_key.insert(key, texts);
            }
            if let Some(text) = self.by_key.get(key).and_then(|texts| texts.get(path)) {
                return Ok(Some(text.clone()));
            }
        }
        Ok(None)
    }
}
