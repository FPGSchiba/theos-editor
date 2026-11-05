//! `schema/required`: every required field is there.

use super::{SchemaRule, Subject};
use crate::finding::{Severity, with_article};
use crate::kinds::Problem;

/// The rule.
pub(super) const RULE: SchemaRule = SchemaRule {
    id: "schema/required",
    severity: Severity::Error,
    run,
};

/// `id` and `kind` have rules of their own.
fn run(subject: &Subject<'_>) -> Vec<Problem> {
    subject
        .spec
        .fields
        .iter()
        .filter(|field| field.required && !matches!(field.key, "id" | "kind"))
        .filter(|field| subject.front.get(field.key).is_none())
        .map(|field| Problem {
            span: subject.kind_span,
            message: format!("{} needs `{}`", with_article(subject.spec.name), field.key),
        })
        .collect()
}
