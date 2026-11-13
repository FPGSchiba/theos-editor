//! The JSON views of the schema: every kind and rule, and the detail of one kind.

use serde::Serialize;

/// Every kind and every rule.
#[derive(Debug, Serialize)]
pub struct SchemaView {
    /// The kinds, in registration order.
    pub kinds: Vec<KindSummaryView>,
    /// The rules, sorted by id.
    pub rules: Vec<RuleView>,
}

/// One kind in the overview.
#[derive(Debug, Serialize)]
pub struct KindSummaryView {
    /// The kind name.
    pub name: String,
    /// The editor part that registered it.
    pub part: String,
    /// The id rule in plain words.
    pub id_rule: String,
    /// Where its records live.
    pub placement: PlacementView,
}

/// Where the records of a kind live.
#[derive(Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum PlacementView {
    /// Flat in this project-relative folder.
    Folder(String),
    /// Under the folder of the record this field links to.
    Under(String),
}

/// One rule in the overview.
#[derive(Debug, Serialize)]
pub struct RuleView {
    /// The rule id, `group/name`.
    pub id: String,
    /// The default severity.
    pub severity: &'static str,
    /// `guardrail`, `configurable` or `fixed`.
    pub class: &'static str,
    /// The severity the project's settings give it; `null` outside a project.
    pub configured: Option<&'static str>,
}

/// One kind in detail.
#[derive(Debug, Serialize)]
pub struct KindView {
    /// The kind name.
    pub name: String,
    /// The editor part that registered it.
    pub part: String,
    /// The id rule in plain words.
    pub id_rule: String,
    /// Where its records live.
    pub placement: PlacementView,
    /// The front-matter fields, in writing order.
    pub fields: Vec<FieldView>,
    /// What the body may contain.
    pub body: BodyView,
}

/// One front-matter field.
#[derive(Debug, Serialize)]
pub struct FieldView {
    /// The field key.
    pub key: String,
    /// Its type in words.
    #[serde(rename = "type")]
    pub ty: String,
    /// Whether a record must have it.
    pub required: bool,
}

/// What the body of a kind may contain.
#[derive(Debug, Serialize)]
pub struct BodyView {
    /// The beat attributes, when the body has beats.
    pub beats: Option<Vec<BeatView>>,
    /// The kinds a note may link to, when the body has notes.
    pub notes: Option<Vec<String>>,
    /// Whether the body is verbatim text.
    pub verbatim: bool,
}

/// One beat attribute.
#[derive(Debug, Serialize)]
pub struct BeatView {
    /// The attribute key.
    pub key: String,
    /// The kinds it may link to.
    pub targets: Vec<String>,
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn placement_is_folder_or_under() -> Result<(), serde_json::Error> {
        let folder = serde_json::to_value(PlacementView::Folder("lore/truths".to_owned()))?;
        let under = serde_json::to_value(PlacementView::Under("parent".to_owned()))?;
        assert_eq!(folder, json!({ "folder": "lore/truths" }));
        assert_eq!(under, json!({ "under": "parent" }));
        Ok(())
    }
}
