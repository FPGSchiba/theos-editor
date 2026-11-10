//! `schema/unknown-field` (warning): a field the kind does not have.

use super::{SchemaRule, Subject};
use crate::finding::{Severity, with_article};
use crate::kinds::Problem;

/// The rule.
pub(super) const RULE: SchemaRule = SchemaRule {
    id: "schema/unknown-field",
    severity: Severity::Warning,
    run,
};

fn run(subject: &Subject<'_>) -> Vec<Problem> {
    subject
        .front
        .fields
        .iter()
        .filter(|field| subject.spec.field(&field.key).is_none())
        .map(|field| Problem {
            span: field.key_span,
            message: format!(
                "`{}` is not a field of {}",
                field.key,
                with_article(subject.spec.name)
            ),
        })
        .collect()
}
