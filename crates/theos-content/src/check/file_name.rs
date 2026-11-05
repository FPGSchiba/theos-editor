//! `index/file-name`: the `id` is the file stem.

use super::{SchemaRule, Subject};
use crate::finding::Severity;
use crate::kinds::Problem;
use crate::yaml::Value;

/// The rule.
pub(super) const RULE: SchemaRule = SchemaRule {
    id: "index/file-name",
    severity: Severity::Error,
    run,
};

fn run(subject: &Subject<'_>) -> Vec<Problem> {
    match subject.front.get("id").map(|field| &field.value) {
        Some(Value::Scalar(scalar)) if scalar.as_text() != subject.id => vec![Problem {
            span: scalar.span,
            message: format!(
                "the id `{}` differs from the file name `{}`",
                scalar.as_text(),
                subject.id
            ),
        }],
        _ => Vec::new(),
    }
}
