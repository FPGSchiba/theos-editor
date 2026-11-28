//! `project::info`: what the MCP tool `project_info` returns.

use std::collections::HashMap;

use crate::check::counts;
use crate::guides;
use crate::schema::overview;
use crate::views::project::{CacheKind, KindCountView, ProjectInfoView, RuleSeverityView};
use crate::{StudioError, Workspace};

/// The project in one answer: its folder, schema version, cache, record counts per kind, the
/// effective severity of every rule, the finding counts, the guides and `tools`.
///
/// # Errors
/// [`StudioError::Index`] when the index fails; [`StudioError::Io`] when the settings file
/// cannot be read.
pub fn info(ws: &Workspace, tools: &[&str]) -> Result<ProjectInfoView, StudioError> {
    let root = ws.project().root();
    let records = ws.index().records()?;
    let mut per_kind: HashMap<&str, usize> = HashMap::new();
    for kind in records.iter().filter_map(|record| record.kind.as_deref()) {
        *per_kind.entry(kind).or_default() += 1;
    }
    let schema = overview(ws.registry(), Some(root))?;
    let (errors, warnings) = counts(&ws.index().findings()?);
    Ok(ProjectInfoView {
        root: root.to_string_lossy().into_owned(),
        schema_version: u32::try_from(theos_content::SCHEMA_VERSION).unwrap_or(u32::MAX),
        cache: if ws.is_in_memory() {
            CacheKind::Memory
        } else {
            CacheKind::File
        },
        records: records.len(),
        kinds: schema
            .kinds
            .into_iter()
            .map(|kind| KindCountView {
                records: per_kind.get(kind.name.as_str()).copied().unwrap_or(0),
                name: kind.name,
                part: kind.part,
            })
            .collect(),
        rules: schema
            .rules
            .into_iter()
            .map(|rule| RuleSeverityView {
                severity: rule.configured.unwrap_or(rule.severity).to_owned(),
                id: rule.id,
            })
            .collect(),
        errors,
        warnings,
        guides: guides::list(root),
        tools: tools.iter().map(|tool| (*tool).to_owned()).collect(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{TestResult, codex_workspace};

    // AC: CE-18.1
    #[test]
    fn project_info_reports() -> TestResult {
        let (_dir, workspace) = codex_workspace()?;
        let view = serde_json::to_value(info(&workspace, &["a", "b"])?)?;
        assert_eq!(view["records"], workspace.index().records()?.len());
        assert_eq!(view["schema_version"], theos_content::SCHEMA_VERSION);
        let npc = view["kinds"]
            .as_array()
            .ok_or("no kinds")?
            .iter()
            .find(|k| k["name"] == "npc")
            .ok_or("no npc")?;
        assert_eq!(npc["records"], 1);
        let rule = view["rules"]
            .as_array()
            .ok_or("no rules")?
            .iter()
            .find(|r| r["id"] == "lore/never-confirm")
            .ok_or("no rule")?;
        assert_eq!(rule["severity"], "error");
        assert_eq!(
            view["guides"],
            serde_json::json!([
                {"name": "atlas", "title": "Placing on the atlas", "path": "tools/guides/atlas.md"},
                {"name": "quest", "title": "quest", "path": "tools/guides/quest.md"},
            ])
        );
        assert_eq!(view["cache"], "file");
        assert_eq!(view["tools"], serde_json::json!(["a", "b"]));
        Ok(())
    }
}
