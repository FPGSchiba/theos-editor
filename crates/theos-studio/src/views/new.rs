//! The JSON view of `theos new`.

use serde::Serialize;

use crate::create::Created;
use crate::views::check::FindingView;

/// The result of `theos new` as JSON.
#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct NewView {
    path: String,
    id: String,
    findings: Vec<FindingView>,
    introduced: Vec<FindingView>,
}

impl From<&Created> for NewView {
    fn from(created: &Created) -> Self {
        NewView {
            path: created.path.clone(),
            id: created.id.clone(),
            findings: created.findings.iter().map(FindingView::from).collect(),
            introduced: created.introduced.iter().map(FindingView::from).collect(),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use theos_content::Finding;

    use super::*;
    use crate::test_support::error;

    #[test]
    fn the_new_view_has_every_key() -> Result<(), Box<dyn std::error::Error>> {
        let refusal: Finding = error("lore/clues/C-901.md", "lore/never-confirm", "synthetic");
        let created = Created {
            path: "lore/clues/C-901.md".to_owned(),
            id: "C-901".to_owned(),
            findings: vec![refusal.clone()],
            introduced: vec![refusal],
        };
        let value = serde_json::to_value(NewView::from(&created))?;
        let keys: BTreeSet<&str> = value
            .as_object()
            .map(|map| map.keys().map(String::as_str).collect())
            .unwrap_or_default();
        assert_eq!(
            keys,
            BTreeSet::from(["path", "id", "findings", "introduced"])
        );
        assert_eq!(value["id"], "C-901");
        assert_eq!(value["introduced"][0]["rule"], "lore/never-confirm");
        assert_eq!(value["findings"][0]["severity"], "error");
        Ok(())
    }
}
