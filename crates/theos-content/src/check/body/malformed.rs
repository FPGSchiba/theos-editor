//! One rule per kind of malformed construct P02 reports, id `body/<kind>`. Each keeps the items
//! that mean something in the kind's grammar.

use super::{BodyInput, BodyRule, problem};
use crate::body::MalformedKind;
use crate::finding::Severity;
use crate::kinds::Problem;

const fn rule(id: &'static str, run: fn(&BodyInput<'_>) -> Vec<Problem>) -> BodyRule {
    BodyRule {
        id,
        severity: Severity::Error,
        run,
    }
}

/// `body/link-unclosed`.
pub(super) const LINK_UNCLOSED: BodyRule = rule("body/link-unclosed", |input| {
    of_kind(input, MalformedKind::LinkUnclosed)
});
/// `body/link-target`.
pub(super) const LINK_TARGET: BodyRule = rule("body/link-target", |input| {
    of_kind(input, MalformedKind::LinkTarget)
});
/// `body/embed`.
pub(super) const EMBED: BodyRule = rule("body/embed", |input| of_kind(input, MalformedKind::Embed));
/// `body/callout-title`: a voice title always, a note title where notes mean something.
pub(super) const CALLOUT_TITLE: BodyRule = rule("body/callout-title", callout_title);
/// `body/beat-heading`.
pub(super) const BEAT_HEADING: BodyRule = rule("body/beat-heading", |input| {
    of_kind(input, MalformedKind::BeatHeading)
});
/// `body/beat-attribute`.
pub(super) const BEAT_ATTRIBUTE: BodyRule = rule("body/beat-attribute", |input| {
    of_kind(input, MalformedKind::BeatAttribute)
});
/// `body/misplaced-attributes`.
pub(super) const MISPLACED_ATTRIBUTES: BodyRule = rule("body/misplaced-attributes", |input| {
    of_kind(input, MalformedKind::MisplacedAttributes)
});

/// The malformed items of `kind`, when the grammar gives that kind of construct a meaning.
fn of_kind(input: &BodyInput<'_>, kind: MalformedKind) -> Vec<Problem> {
    let means = match kind {
        MalformedKind::BeatHeading
        | MalformedKind::BeatAttribute
        | MalformedKind::MisplacedAttributes => input.grammar.beats.is_some(),
        _ => true,
    };
    if !means {
        return Vec::new();
    }
    input
        .body
        .malformed()
        .iter()
        .filter(|item| item.kind == kind)
        .map(|item| problem(item.span, item.message.clone()))
        .collect()
}

/// Malformed callout titles; the type of each is the type of the callout whose title holds it.
fn callout_title(input: &BodyInput<'_>) -> Vec<Problem> {
    let means = |name: &str| match name {
        "voice" => true,
        "note" => input.grammar.notes.is_some(),
        _ => false,
    };
    input
        .body
        .malformed()
        .iter()
        .filter(|item| item.kind == MalformedKind::CalloutTitle)
        .filter(|item| {
            input
                .body
                .callouts()
                .iter()
                .find(|c| {
                    c.title_span.start <= item.span.start && item.span.end <= c.title_span.end
                })
                .is_some_and(|c| means(&c.name))
        })
        .map(|item| problem(item.span, item.message.clone()))
        .collect()
}
