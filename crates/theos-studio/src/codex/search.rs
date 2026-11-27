//! `codex::search`: full-text search with the marks of each hit.

use theos_content::{Column, Hit, Match, read_record};

use super::lookup::{Lookup, saturate};
use crate::views::codex::{HitView, MatchView, SearchView};
use crate::{StudioError, Workspace};

/// The most hits one search returns.
const MAX_LIMIT: usize = 100;

/// The records matching `query`, best first, at most `limit` (1 to 100), each with its marks.
///
/// # Errors
/// [`StudioError::InvalidFilter`] for a `limit` out of range; [`StudioError::Index`] when the
/// index fails.
pub fn search(ws: &Workspace, query: &str, limit: usize) -> Result<SearchView, StudioError> {
    if !(1..=MAX_LIMIT).contains(&limit) {
        return Err(StudioError::InvalidFilter {
            message: format!("limit must be between 1 and {MAX_LIMIT}, not {limit}"),
        });
    }
    let lookup = Lookup::load(ws)?;
    // One more than asked for tells whether the list was cut.
    let mut hits = ws.index().search(query, limit + 1)?;
    let truncated = hits.len() > limit;
    hits.truncate(limit);
    Ok(SearchView {
        query: query.to_owned(),
        hits: hits.iter().map(|hit| hit_view(ws, hit, &lookup)).collect(),
        truncated,
    })
}

fn hit_view(ws: &Workspace, hit: &Hit, lookup: &Lookup) -> HitView {
    // The file is read once; a file that cannot be read gives empty `match` texts.
    let record = match read_record(&ws.project().root().join(&hit.path)) {
        Ok(record) => Some(record),
        Err(error) => {
            tracing::warn!(path = %hit.path, %error, "a hit's record cannot be read");
            None
        }
    };
    HitView {
        record: lookup.of_hit(hit),
        score: hit.score,
        matches: hit
            .matches
            .iter()
            .map(|found| {
                let text = record
                    .as_ref()
                    .and_then(|record| record.text().get(found.span.start..found.span.end));
                match_view(found, text.unwrap_or_default())
            })
            .collect(),
    }
}

fn match_view(found: &Match, text: &str) -> MatchView {
    MatchView {
        column: column_name(&found.column),
        line: saturate(found.position.line),
        column_number: saturate(found.position.column),
        r#match: text.to_owned(),
        text: found.line.clone(),
    }
}

fn column_name(column: &Column) -> String {
    match column {
        Column::Title => "title".to_owned(),
        Column::Body => "body".to_owned(),
        Column::Field { key, entry: None } => format!("field:{key}"),
        Column::Field {
            key,
            entry: Some(entry),
        } => format!("field:{key}.{entry}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::StudioError;
    use crate::test_support::{TestResult, codex_workspace};

    // AC: CE-18.1
    #[test]
    fn search_finds_mother_ilse() -> TestResult {
        let (_dir, ws) = codex_workspace()?;
        let view = search(&ws, "Mother Ilse", 20)?;
        let first = view.hits.first().ok_or("no hits")?;
        assert_eq!(first.record.id, "mother-ilse");
        let title = first
            .matches
            .iter()
            .find(|m| m.column == "title")
            .ok_or("no title match")?;
        assert!(title.line >= 1);
        assert_ne!(title.r#match, "");
        assert!("Mother Ilse".contains(&title.r#match), "{}", title.r#match);
        assert!(!view.truncated);
        Ok(())
    }

    // AC: CE-18.1
    #[test]
    fn search_hit_carries_status() -> TestResult {
        let (_dir, ws) = codex_workspace()?;
        let view = search(&ws, "Synthetic cult", 20)?;
        let hit = view
            .hits
            .iter()
            .find(|hit| hit.record.id == "fates-cult")
            .ok_or("fates-cult not hit")?;
        assert_eq!(hit.record.status.as_deref(), Some("canon"));
        let json = serde_json::to_value(&view)?;
        assert!(
            json["hits"]
                .as_array()
                .ok_or("hits")?
                .iter()
                .any(|h| h["status"] == "canon")
        );
        Ok(())
    }

    // AC: CE-18.1
    // AC: CE-18.3
    #[test]
    fn search_marks_never_truth() -> TestResult {
        let (_dir, ws) = codex_workspace()?;
        let view = search(&ws, "Synthetic statement A", 20)?;
        let json = serde_json::to_value(&view)?;
        let hit = json["hits"]
            .as_array()
            .ok_or("hits")?
            .iter()
            .find(|hit| hit["id"] == "T-911")
            .ok_or("T-911 not hit")?;
        assert_eq!(hit["never"], true);
        assert_eq!(hit["locked"], serde_json::json!(["never"]));
        Ok(())
    }

    // AC: CE-18.1
    #[test]
    fn search_limit_bounds() -> TestResult {
        let (_dir, ws) = codex_workspace()?;
        for limit in [0, 101] {
            let error = search(&ws, "Synthetic", limit).err().ok_or("accepted")?;
            assert!(
                matches!(error, StudioError::InvalidFilter { .. }),
                "{error:?}"
            );
        }
        let one = search(&ws, "Synthetic", 1)?;
        assert_eq!(one.hits.len(), 1);
        assert!(one.truncated);
        let all = search(&ws, "Synthetic", 100)?;
        assert!(all.hits.len() > 1);
        assert!(!all.truncated);
        Ok(())
    }
}
