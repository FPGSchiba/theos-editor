//! `schema/kind`: the record names a registered kind.

use crate::kinds::{KindRegistry, KindSpec, Problem, ValueRef};
use crate::text::Span;
use crate::yaml::{FrontMatter, Value};

/// The rule id.
pub(super) const RULE: &str = "schema/kind";

/// The kind of the record and the span of its `kind` value, or what is wrong with it.
pub(super) fn run<'a>(
    registry: &'a KindRegistry,
    front: &FrontMatter,
) -> Result<(&'a KindSpec, Span), Problem> {
    let Some(field) = front.get("kind") else {
        return Err(Problem {
            span: Span::default(),
            message: "a record needs `kind`".to_owned(),
        });
    };
    let span = ValueRef::of(&field.value).span();
    let known = || {
        registry
            .kinds()
            .map(|spec| spec.name)
            .collect::<Vec<_>>()
            .join(", ")
    };
    let Value::Scalar(scalar) = &field.value else {
        return Err(Problem {
            span,
            message: format!("`kind` must be one of {}", known()),
        });
    };
    match registry.kind(scalar.as_text()) {
        Some(spec) => Ok((spec, span)),
        None => Err(Problem {
            span,
            message: format!(
                "`{}` is not a kind; the kinds are {}",
                scalar.as_text(),
                known()
            ),
        }),
    }
}
