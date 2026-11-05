//! What concept versions and map versions share (data model §8).

use crate::kinds::{CheckInput, FieldSpec, FieldType, KindCheck, Problem, Target};
use crate::yaml::Value;

use super::common::field;

/// The fields of a version after the common ones; `parent` is `concept` or `map`, and the
/// parent field links to the kind of that name.
pub(crate) fn fields(
    parent: &'static str,
    own_kind: &'static [Target],
    with_state: bool,
) -> Vec<FieldSpec> {
    const CONCEPT: &[Target] = &[Target::kind("concept")];
    const MAP: &[Target] = &[Target::kind("map")];
    let parent_target = if parent == "concept" { CONCEPT } else { MAP };
    let mut own = vec![
        field(parent, FieldType::Link(parent_target), true),
        field("generator", FieldType::Generator, true),
        field("recipe", FieldType::Text, true),
        field(
            "seed",
            FieldType::Integer {
                min: i64::MIN,
                max: i64::MAX,
            },
            false,
        ),
        field("inputs", FieldType::OpenMap, false),
        field("from", FieldType::Link(own_kind), false),
        field("outputs", FieldType::BinaryRefs, true),
        field("results", FieldType::OpenMap, false),
        field("addresses", FieldType::NoteIds, false),
        field(
            "rating",
            FieldType::Enum(&["unrated", "keep", "kill", "mix"]),
            true,
        ),
    ];
    if with_state {
        own.push(field("state", FieldType::Enum(&["T1", "T3"]), false));
    }
    own
}

/// `schema/version-id`: the id is the parent's id, `-v` and a number.
pub(crate) const VERSION_ID: KindCheck = KindCheck {
    rule: "schema/version-id",
    run: version_id,
};

fn scalar_text<'a>(input: &'a CheckInput<'_>, key: &str) -> Option<&'a str> {
    match &input.front.get(key)?.value {
        Value::Scalar(scalar) => Some(scalar.as_text()),
        _ => None,
    }
}

fn version_id(input: &CheckInput<'_>) -> Vec<Problem> {
    let parent_key = match scalar_text(input, "kind") {
        Some("map-version") => "map",
        _ => "concept",
    };
    let Some(parent) = scalar_text(input, parent_key) else {
        return Vec::new();
    };
    let number = input
        .id
        .strip_prefix(parent)
        .and_then(|rest| rest.strip_prefix("-v"));
    if number.is_some_and(|n| !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit())) {
        return Vec::new();
    }
    let span = input
        .front
        .get("id")
        .map_or(input.kind_span, |id| match &id.value {
            Value::Scalar(scalar) => scalar.span,
            _ => input.kind_span,
        });
    vec![Problem {
        span,
        message: format!(
            "the id of a version is its {parent_key}'s id, `-v` and a number: `{parent}-v<number>`"
        ),
    }]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::record;
    use crate::text::Span;

    /// Runs the check on `front`, with the id taken from `id_text`.
    fn run(front: &str, id_text: &str) -> Result<Vec<Problem>, Box<dyn std::error::Error>> {
        let record = record(front, "");
        let front = record.front_matter().map_err(ToString::to_string)?;
        let input = CheckInput {
            id: id_text,
            front,
            kind_span: Span::new(1, 2),
        };
        Ok((VERSION_ID.run)(&input))
    }

    // AC: CE-11.1
    #[test]
    fn a_version_id_is_its_parent_and_a_number() -> Result<(), Box<dyn std::error::Error>> {
        let concept = "id: fates-shrine-v5\nkind: concept-version\nconcept: fates-shrine\n";
        assert_eq!(run(concept, "fates-shrine-v5")?, Vec::new());
        let map = "id: f04-terrain-v3\nkind: map-version\nmap: f04-terrain\n";
        assert_eq!(run(map, "f04-terrain-v3")?, Vec::new());

        let other = "id: fates-shrine-v5\nkind: concept-version\nconcept: other\n";
        let wrong = run(other, "fates-shrine-v5")?;
        assert_eq!(wrong.len(), 1);
        assert_eq!(wrong[0].span, Span::new(8, 23));
        let other_map = "id: f04-terrain-v3\nkind: map-version\nmap: other\n";
        assert_eq!(run(other_map, "f04-terrain-v3")?.len(), 1);
        for bad in [
            "fates-shrine",
            "fates-shrine-v",
            "fates-shrine-vx",
            "fates-shrine-v5a",
        ] {
            let front = format!("id: {bad}\nkind: concept-version\nconcept: fates-shrine\n");
            assert_eq!(run(&front, bad)?.len(), 1, "{bad}");
        }

        assert_eq!(
            run("id: x-v1\nkind: concept-version\n", "x-v1")?,
            Vec::new()
        );
        let listed = "id: x-v1\nkind: concept-version\nconcept: [a]\n";
        assert_eq!(run(listed, "x-v1")?, Vec::new());
        Ok(())
    }

    // AC: CE-11.1
    #[test]
    fn a_version_without_a_scalar_id_is_reported_at_the_kind()
    -> Result<(), Box<dyn std::error::Error>> {
        let none = run("kind: concept-version\nconcept: a\n", "")?;
        assert_eq!(none.len(), 1);
        assert_eq!(none[0].span, Span::new(1, 2));
        let listed = run("id: [a]\nkind: concept-version\nconcept: a\n", "a")?;
        assert_eq!(listed[0].span, Span::new(1, 2));
        Ok(())
    }
}
