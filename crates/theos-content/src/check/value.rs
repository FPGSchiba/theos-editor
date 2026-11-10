//! `schema/value`: every field the kind knows holds a value of its type.

use super::{SchemaRule, Subject};
use crate::finding::Severity;
use crate::kinds::{FieldType, Problem, ValueRef};

/// The rule.
pub(super) const RULE: SchemaRule = SchemaRule {
    id: "schema/value",
    severity: Severity::Error,
    run,
};

/// `id` and `kind` are judged by their own rules.
fn run(subject: &Subject<'_>) -> Vec<Problem> {
    subject
        .spec
        .fields
        .iter()
        .filter(|field| !matches!(field.ty, FieldType::Id | FieldType::KindName))
        .filter_map(|field| {
            let present = subject.front.get(field.key)?;
            Some(field.check(ValueRef::of(&present.value)))
        })
        .flatten()
        .collect()
}
