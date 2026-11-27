//! The JSON views of the Codex queries.

use schemars::JsonSchema;
use serde::Serialize;
use theos_content::RecordSummary;

use crate::marks::{MarkView, Marks};

/// A record named in a result: who it is, where it lies and what marks it carries.
#[derive(Clone, Debug, Serialize, JsonSchema)]
pub struct RecordRef {
    /// The id (the file stem).
    pub id: String,
    /// The project-relative path with `/`; `null` when no record has the id.
    pub path: Option<String>,
    /// The kind, when the front matter names a registered one.
    pub kind: Option<String>,
    /// The text of the `title` scalar.
    pub title: Option<String>,
    /// The text of the `status` scalar.
    pub status: Option<String>,
    /// `never` and `locked`, flattened next to the id.
    #[serde(flatten)]
    pub mark: MarkView,
}

impl RecordRef {
    /// The reference to the record `summary` describes, with the marks of its path.
    #[must_use]
    pub fn from_summary(summary: &RecordSummary, marks: &Marks) -> RecordRef {
        RecordRef {
            id: summary.id.clone(),
            path: Some(summary.path.clone()),
            kind: summary.kind.clone(),
            title: summary.title.clone(),
            status: summary.status.clone(),
            mark: marks.of_path(&summary.path),
        }
    }

    /// The reference to an id no record has: every key but `id` is empty and no mark is set.
    #[must_use]
    pub fn missing(id: &str) -> RecordRef {
        RecordRef {
            id: id.to_owned(),
            path: None,
            kind: None,
            title: None,
            status: None,
            mark: MarkView::default(),
        }
    }
}

/// The result of `codex::search`.
#[derive(Clone, Debug, Serialize, JsonSchema)]
pub struct SearchView {
    /// The query as given.
    pub query: String,
    /// The hits, best first.
    pub hits: Vec<HitView>,
    /// Whether more records match than `hits` holds.
    pub truncated: bool,
}

/// One record that matches a query.
#[derive(Clone, Debug, Serialize, JsonSchema)]
pub struct HitView {
    /// The record.
    #[serde(flatten)]
    pub record: RecordRef,
    /// The relevance: higher is better.
    pub score: f64,
    /// Where the words match.
    pub matches: Vec<MatchView>,
}

/// One place where a query matches a record.
#[derive(Clone, Debug, Serialize, JsonSchema)]
pub struct MatchView {
    /// `title`, `body`, `field:<key>` or `field:<key>.<entry>`.
    pub column: String,
    /// The line of the match, from 1.
    pub line: u32,
    /// The column of the match, from 1, in Unicode scalar values.
    pub column_number: u32,
    /// The record text under the match; empty when the file could not be read.
    #[serde(rename = "match")]
    pub r#match: String,
    /// The line of the record that holds the match.
    pub text: String,
}

/// The result of `codex::list`.
#[derive(Clone, Debug, Serialize, JsonSchema)]
pub struct ListView {
    /// The page of records, ordered by path.
    pub records: Vec<RecordRef>,
    /// How many records pass the filter.
    pub total: usize,
    /// Where the page starts.
    pub offset: usize,
    /// Whether records follow the page.
    pub truncated: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Marks;
    use crate::test_support::{TestResult, codex_workspace};

    // AC: CE-18.3
    #[test]
    fn record_ref_json_keys() -> TestResult {
        let (_dir, ws) = codex_workspace()?;
        let marks = Marks::load(ws.index())?;
        let summary = ws
            .index()
            .records()?
            .into_iter()
            .find(|r| r.id == "T-911")
            .ok_or("T-911")?;
        let json = serde_json::to_value(RecordRef::from_summary(&summary, &marks))?;
        let object = json.as_object().ok_or("object")?;
        let mut keys: Vec<&str> = object.keys().map(String::as_str).collect();
        keys.sort_unstable();
        assert_eq!(
            keys,
            ["id", "kind", "locked", "never", "path", "status", "title"]
        );
        assert_eq!(json["never"], true);

        let json = serde_json::to_value(RecordRef::missing("ghost"))?;
        assert_eq!(
            json,
            serde_json::json!({"id": "ghost", "path": null, "kind": null, "title": null,
                "status": null, "never": false, "locked": []})
        );
        Ok(())
    }
}
