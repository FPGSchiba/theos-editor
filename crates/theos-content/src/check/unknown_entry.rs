//! `schema/unknown-entry` (warning): a lens entry that no lens has.

use super::{SchemaRule, Subject};
use crate::finding::Severity;
use crate::kinds::{FieldType, LENS_ENTRIES, Problem};
use crate::yaml::Value;

/// The rule.
pub(super) const RULE: SchemaRule = SchemaRule {
    id: "schema/unknown-entry",
    severity: Severity::Warning,
    run,
};

fn run(subject: &Subject<'_>) -> Vec<Problem> {
    subject
        .spec
        .fields
        .iter()
        .filter(|field| field.ty == FieldType::Lens)
        .filter_map(|field| match &subject.front.get(field.key)?.value {
            Value::Map(map) => Some((field.key, map)),
            _ => None,
        })
        .flat_map(|(key, map)| {
            map.entries
                .iter()
                .filter(|entry| LENS_ENTRIES.iter().all(|known| known.key != entry.key))
                .map(move |entry| Problem {
                    span: entry.key_span,
                    message: format!("`{}` is not an entry of `{key}`", entry.key),
                })
        })
        .collect()
}
