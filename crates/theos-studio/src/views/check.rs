//! The JSON views of a check and of a staged check.

use serde::Serialize;
use theos_content::Finding;

use crate::check::{Binaries, Check, counts};
use crate::staged::StagedCheck;

/// A finding as JSON: every key is present, an absent one is `null`.
#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct FindingView {
    rule: String,
    severity: &'static str,
    path: String,
    id: Option<String>,
    line: Option<usize>,
    column: Option<usize>,
    span: Option<SpanView>,
    message: String,
}

/// A byte range as JSON.
#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct SpanView {
    start: usize,
    end: usize,
}

/// Whether binaries were checked, as JSON.
#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct BinariesView {
    checked: bool,
    folder: Option<String>,
    reason: Option<String>,
}

/// The result of a check as JSON.
#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct CheckView {
    findings: Vec<FindingView>,
    errors: usize,
    warnings: usize,
    binaries: BinariesView,
}

/// The result of a staged check as JSON.
#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct StagedView {
    findings: Vec<FindingView>,
    errors: usize,
    warnings: usize,
    changed: Vec<String>,
}

impl From<&Finding> for FindingView {
    fn from(finding: &Finding) -> Self {
        FindingView {
            rule: finding.rule.clone(),
            severity: finding.severity.as_str(),
            path: finding.path.clone(),
            id: finding.id.clone(),
            line: finding.position.map(|p| p.line),
            column: finding.position.map(|p| p.column),
            span: finding.span.map(|s| SpanView {
                start: s.start,
                end: s.end,
            }),
            message: finding.message.clone(),
        }
    }
}

impl From<&Check> for CheckView {
    fn from(check: &Check) -> Self {
        let (errors, warnings) = counts(&check.findings);
        let binaries = match &check.binaries {
            Binaries::Checked { folder } => BinariesView {
                checked: true,
                folder: Some(folder.to_string_lossy().into_owned()),
                reason: None,
            },
            Binaries::NotChecked { reason } => BinariesView {
                checked: false,
                folder: None,
                reason: Some(reason.clone()),
            },
        };
        CheckView {
            findings: check.findings.iter().map(FindingView::from).collect(),
            errors,
            warnings,
            binaries,
        }
    }
}

impl From<&StagedCheck> for StagedView {
    fn from(staged: &StagedCheck) -> Self {
        StagedView {
            findings: staged.findings.iter().map(FindingView::from).collect(),
            errors: staged.errors(),
            warnings: staged.warnings(),
            changed: staged.changed.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use theos_content::{Finding, Position, Severity, Span};

    use super::*;
    use crate::check::{Binaries, Check};
    use crate::staged::StagedCheck;

    fn finding(id: Option<&str>, at: Option<(Span, Position)>) -> Finding {
        Finding {
            rule: "binary/missing".to_owned(),
            severity: Severity::Warning,
            path: "world/a.md".to_owned(),
            id: id.map(str::to_owned),
            span: at.map(|(span, _)| span),
            position: at.map(|(_, position)| position),
            message: "synthetic".to_owned(),
        }
    }

    fn keys(value: &serde_json::Value) -> BTreeSet<&str> {
        value
            .as_object()
            .map(|map| map.keys().map(String::as_str).collect())
            .unwrap_or_default()
    }

    fn at() -> (Span, Position) {
        (Span { start: 3, end: 9 }, Position { line: 2, column: 4 })
    }

    #[test]
    fn a_finding_without_a_position_has_null_line_column_and_span()
    -> Result<(), Box<dyn std::error::Error>> {
        let value = serde_json::to_value(FindingView::from(&finding(Some("a"), None)))?;
        assert!(value["line"].is_null());
        assert!(value["column"].is_null());
        assert!(value["span"].is_null());
        assert_eq!(value["severity"], "warning");
        Ok(())
    }

    #[test]
    fn a_finding_with_a_position_has_line_column_and_span() -> Result<(), Box<dyn std::error::Error>>
    {
        let value = serde_json::to_value(FindingView::from(&finding(Some("a"), Some(at()))))?;
        assert_eq!(value["line"], 2);
        assert_eq!(value["column"], 4);
        assert_eq!(value["span"], serde_json::json!({"start": 3, "end": 9}));
        assert_eq!(value["id"], "a");
        Ok(())
    }

    #[test]
    fn a_finding_on_an_input_file_has_a_null_id() -> Result<(), Box<dyn std::error::Error>> {
        let value = serde_json::to_value(FindingView::from(&finding(None, Some(at()))))?;
        assert!(value["id"].is_null());
        assert_eq!(value["path"], "world/a.md");
        Ok(())
    }

    #[test]
    fn the_check_view_has_every_key() -> Result<(), Box<dyn std::error::Error>> {
        let checked = Check {
            findings: vec![finding(None, None)],
            binaries: Binaries::Checked {
                folder: "bin".into(),
            },
        };
        let value = serde_json::to_value(CheckView::from(&checked))?;
        assert_eq!(
            keys(&value),
            BTreeSet::from(["findings", "errors", "warnings", "binaries"])
        );
        assert_eq!(value["warnings"], 1);
        assert_eq!(value["errors"], 0);
        assert_eq!(
            keys(&value["findings"][0]),
            BTreeSet::from([
                "rule", "severity", "path", "id", "line", "column", "span", "message"
            ])
        );
        let expected = BTreeSet::from(["checked", "folder", "reason"]);
        assert_eq!(keys(&value["binaries"]), expected);
        assert_eq!(value["binaries"]["checked"], true);
        assert_eq!(value["binaries"]["folder"], "bin");
        assert!(value["binaries"]["reason"].is_null());

        let skipped = Check {
            findings: Vec::new(),
            binaries: Binaries::NotChecked {
                reason: "no binaries folder set".to_owned(),
            },
        };
        let value = serde_json::to_value(CheckView::from(&skipped))?;
        assert_eq!(keys(&value["binaries"]), expected);
        assert_eq!(value["binaries"]["checked"], false);
        assert!(value["binaries"]["folder"].is_null());
        assert_eq!(value["binaries"]["reason"], "no binaries folder set");
        Ok(())
    }

    #[test]
    fn the_staged_view_has_changed_and_no_binaries() -> Result<(), Box<dyn std::error::Error>> {
        let mut error = finding(Some("a"), Some(at()));
        error.severity = Severity::Error;
        let staged = StagedCheck {
            findings: vec![error, finding(None, None)],
            changed: vec!["world/a.md".to_owned()],
        };
        let value = serde_json::to_value(StagedView::from(&staged))?;
        assert_eq!(
            keys(&value),
            BTreeSet::from(["findings", "errors", "warnings", "changed"])
        );
        assert_eq!(value["errors"], 1);
        assert_eq!(value["warnings"], 1);
        assert_eq!(value["changed"], serde_json::json!(["world/a.md"]));
        assert_eq!(value["findings"][0]["severity"], "error");
        Ok(())
    }
}
