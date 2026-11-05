//! The `topic` kind.

use crate::kinds::{BodyGrammar, CheckInput, FieldType, IdRule, KindCheck, KindSpec, Placement};
use crate::kinds::{Problem, Target};
use crate::parts::common::{self, field};
use crate::yaml::Value;

const EXPLAINING: &[Target] = &[Target::narrowed("topic", "track", "explaining")];

/// The spec of the `topic` kind.
pub(crate) fn spec() -> KindSpec {
    KindSpec {
        name: "topic",
        id: IdRule {
            pattern: "^[a-z0-9]+(-[a-z0-9]+)*-(explaining|in-world)$",
            says: "a slug ending in `-explaining` or `-in-world`",
        },
        placement: Placement::Flat("lore/topics"),
        fields: common::fields(
            true,
            &[
                field("track", FieldType::Enum(&["explaining", "in-world"]), true),
                field("counterpart", FieldType::Link(EXPLAINING), false),
                field("source", FieldType::Url, false),
            ],
        ),
        body: BodyGrammar::PROSE,
        checks: vec![
            KindCheck {
                rule: "schema/topic-track",
                run: track_matches_id,
            },
            KindCheck {
                rule: "schema/topic-counterpart",
                run: in_world_has_counterpart,
            },
        ],
    }
}

/// `schema/topic-track`: a valid `track` equals the suffix of the id.
fn track_matches_id(input: &CheckInput<'_>) -> Vec<Problem> {
    let Some(field) = input.front.get("track") else {
        return Vec::new();
    };
    let Value::Scalar(track) = &field.value else {
        return Vec::new();
    };
    let suffix = ["explaining", "in-world"]
        .into_iter()
        .find(|s| input.id.ends_with(&format!("-{s}")));
    match suffix {
        Some(suffix)
            if matches!(track.as_text(), "explaining" | "in-world")
                && track.as_text() != suffix =>
        {
            vec![Problem {
                span: track.span,
                message: format!(
                    "the track is `{}` but the id ends in `-{suffix}`",
                    track.as_text()
                ),
            }]
        }
        _ => Vec::new(),
    }
}

/// `schema/topic-counterpart`: an in-world topic names the explaining topic it converges on.
fn in_world_has_counterpart(input: &CheckInput<'_>) -> Vec<Problem> {
    if input.id.ends_with("-in-world") && input.front.get("counterpart").is_none() {
        vec![Problem {
            span: input.kind_span,
            message: "an in-world topic needs a `counterpart`".to_owned(),
        }]
    } else {
        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::record;
    use crate::text::Span;

    fn run(
        check: fn(&CheckInput<'_>) -> Vec<Problem>,
        id: &str,
        front: &str,
    ) -> Result<Vec<Problem>, Box<dyn std::error::Error>> {
        let record = record(front, "");
        let front = record.front_matter().map_err(ToString::to_string)?;
        Ok(check(&CheckInput {
            id,
            front,
            kind_span: Span::new(1, 2),
        }))
    }

    // AC: CE-11.1
    #[test]
    fn topic_track_matches_the_id_suffix() -> Result<(), Box<dyn std::error::Error>> {
        let fine = run(track_matches_id, "runes-in-world", "track: in-world\n")?;
        assert_eq!(fine, Vec::new());
        let fine = run(track_matches_id, "runes-explaining", "track: explaining\n")?;
        assert_eq!(fine, Vec::new());

        let wrong = run(track_matches_id, "runes-in-world", "track: explaining\n")?;
        assert_eq!(wrong.len(), 1);
        assert_eq!(wrong[0].span, Span::new(11, 21));

        // Not a valid track, a missing track, a list and an id without suffix are other findings.
        assert_eq!(
            run(track_matches_id, "runes-in-world", "track: nope\n")?,
            Vec::new()
        );
        assert_eq!(
            run(track_matches_id, "runes-in-world", "title: a\n")?,
            Vec::new()
        );
        assert_eq!(
            run(track_matches_id, "runes-in-world", "track: [a]\n")?,
            Vec::new()
        );
        assert_eq!(
            run(track_matches_id, "runes", "track: explaining\n")?,
            Vec::new()
        );
        Ok(())
    }

    // AC: CE-11.1
    #[test]
    fn an_in_world_topic_needs_a_counterpart() -> Result<(), Box<dyn std::error::Error>> {
        let missing = run(
            in_world_has_counterpart,
            "runes-in-world",
            "track: in-world\n",
        )?;
        assert_eq!(missing.len(), 1);
        assert_eq!(missing[0].span, Span::new(1, 2));
        let given = "track: in-world\ncounterpart: runes-explaining\n";
        assert_eq!(
            run(in_world_has_counterpart, "runes-in-world", given)?,
            Vec::new()
        );
        let explaining = run(
            in_world_has_counterpart,
            "runes-explaining",
            "track: explaining\n",
        )?;
        assert_eq!(explaining, Vec::new());
        Ok(())
    }
}
