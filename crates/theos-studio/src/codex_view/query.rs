//! The query syntax of the Codex find field (P11 §5.10): `#<text>` keeps records whose id starts
//! with the text, ignoring case; `:<kind>` keeps records of that kind; `@<text>` keeps records of
//! the Who tab's kinds and searches the text as a word; every other word goes to P03's search
//! through `codex::search`. Prefixes combine; with no word left, the filters list records by id.

use serde::Serialize;
use theos_content::RecordSummary;

use super::tree::WHO;
use crate::codex::search;
use crate::views::codex::{HitView, RecordRef};
use crate::{Marks, StudioError, Workspace};

/// The most hits one query returns, as `codex::search` allows.
const MAX_LIMIT: u32 = 100;

/// The result of a query.
#[derive(Clone, Debug, Serialize)]
pub struct QueryView {
    /// The hits: best first for words, by id for filters alone.
    pub hits: Vec<HitView>,
    /// Whether more records match than `hits` holds.
    pub truncated: bool,
    /// A line for the user, such as "no kind `x`".
    pub note: Option<String>,
}

/// One filter of a query.
#[derive(Debug, PartialEq, Eq)]
enum Filter {
    /// The id starts with this text, in lower case.
    IdPrefix(String),
    /// The record has this kind.
    Kind(String),
    /// The record has a kind of the Who tab.
    Who,
}

impl Filter {
    fn keeps(&self, record: &RecordRef) -> bool {
        let kind = record.kind.as_deref();
        match self {
            Filter::IdPrefix(prefix) => record.id.to_lowercase().starts_with(prefix),
            Filter::Kind(name) => kind == Some(name.as_str()),
            Filter::Who => kind.is_some_and(|kind| WHO.contains(&kind)),
        }
    }
}

/// A query split into its filters and the words for search.
#[derive(Debug, PartialEq, Eq)]
struct Parsed {
    filters: Vec<Filter>,
    words: String,
    /// A kind after `:` that the registry does not know.
    unknown_kind: Option<String>,
}

/// The records `text` finds, at most `limit` (1 to 100; a larger limit counts as 100). The caller
/// syncs the index first.
///
/// # Errors
/// [`StudioError::Index`] when the index fails.
pub fn query(ws: &Workspace, text: &str, limit: u32) -> Result<QueryView, StudioError> {
    let parsed = parse(ws, text);
    if let Some(kind) = parsed.unknown_kind {
        return Ok(QueryView {
            hits: Vec::new(),
            truncated: false,
            note: Some(format!("no kind `{kind}`")),
        });
    }
    let limit = usize::try_from(limit.clamp(1, MAX_LIMIT)).unwrap_or(1);
    if !parsed.words.is_empty() {
        return by_words(ws, &parsed, limit);
    }
    if parsed.filters.is_empty() {
        return Ok(QueryView {
            hits: Vec::new(),
            truncated: false,
            note: None,
        });
    }
    by_filters(ws, &parsed.filters, limit)
}

/// Splits `text` at whitespace into prefixes and words. A prefix with no text after it filters
/// nothing, except `@`, which keeps the Who kinds.
fn parse(ws: &Workspace, text: &str) -> Parsed {
    let mut parsed = Parsed {
        filters: Vec::new(),
        words: String::new(),
        unknown_kind: None,
    };
    let mut words = Vec::new();
    for token in text.split_whitespace() {
        if let Some(prefix) = token.strip_prefix('#') {
            if !prefix.is_empty() {
                parsed.filters.push(Filter::IdPrefix(prefix.to_lowercase()));
            }
        } else if let Some(kind) = token.strip_prefix(':') {
            if kind.is_empty() {
                continue;
            }
            match ws
                .registry()
                .kinds()
                .find(|spec| spec.name.eq_ignore_ascii_case(kind))
            {
                Some(spec) => parsed.filters.push(Filter::Kind(spec.name.to_owned())),
                None => {
                    parsed.unknown_kind.get_or_insert_with(|| kind.to_owned());
                }
            }
        } else if let Some(word) = token.strip_prefix('@') {
            if !parsed.filters.contains(&Filter::Who) {
                parsed.filters.push(Filter::Who);
            }
            if !word.is_empty() {
                words.push(word);
            }
        } else {
            words.push(token);
        }
    }
    parsed.words = words.join(" ");
    parsed
}

/// The records the words find, kept by the filters. The filters run on P03's best hits, at most
/// [`MAX_LIMIT`] of them: on a large project a filter with a common word can miss a record that
/// ranks lower (P11 ruling; filtering inside the search is P03's and P48's code).
fn by_words(ws: &Workspace, parsed: &Parsed, limit: usize) -> Result<QueryView, StudioError> {
    // With filters, search the most search allows, so the filters keep enough hits.
    let fetch = if parsed.filters.is_empty() {
        limit
    } else {
        usize::try_from(MAX_LIMIT).unwrap_or(limit)
    };
    let found = search(ws, &parsed.words, fetch)?;
    let mut hits: Vec<HitView> = found
        .hits
        .into_iter()
        .filter(|hit| parsed.filters.iter().all(|f| f.keeps(&hit.record)))
        .collect();
    let truncated = hits.len() > limit || found.truncated;
    hits.truncate(limit);
    Ok(QueryView {
        hits,
        truncated,
        note: None,
    })
}

fn by_filters(ws: &Workspace, filters: &[Filter], limit: usize) -> Result<QueryView, StudioError> {
    let marks = Marks::load(ws.index())?;
    let mut records: Vec<RecordSummary> = ws.index().records()?;
    records.sort_by(|a, b| a.id.cmp(&b.id).then_with(|| a.path.cmp(&b.path)));
    let mut hits: Vec<HitView> = records
        .iter()
        .map(|record| RecordRef::from_summary(record, &marks))
        .filter(|record| filters.iter().all(|f| f.keeps(record)))
        .map(|record| HitView {
            record,
            score: 0.0,
            matches: Vec::new(),
        })
        .collect();
    let truncated = hits.len() > limit;
    hits.truncate(limit);
    Ok(QueryView {
        hits,
        truncated,
        note: None,
    })
}
