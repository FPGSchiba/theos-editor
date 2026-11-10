//! `schema/id`: a record has an `id`, and the file stem fits the id rule of its kind.

use super::{SchemaRule, Subject};
use crate::finding::Severity;
use crate::kinds::{Problem, ValueRef};
use crate::yaml::Value;

/// The rule.
pub(super) const RULE: SchemaRule = SchemaRule {
    id: "schema/id",
    severity: Severity::Error,
    run,
};

fn run(subject: &Subject<'_>) -> Vec<Problem> {
    let Some(field) = subject.front.get("id") else {
        return vec![Problem {
            span: subject.kind_span,
            message: "a record needs `id`".to_owned(),
        }];
    };
    let Value::Scalar(scalar) = &field.value else {
        return vec![Problem {
            span: ValueRef::of(&field.value).span(),
            message: "`id` must be a text".to_owned(),
        }];
    };
    if subject.registry.id_fits(subject.spec.name, subject.id) {
        Vec::new()
    } else {
        vec![Problem {
            span: scalar.span,
            message: format!("the id `{}` must be {}", subject.id, subject.spec.id.says),
        }]
    }
}
