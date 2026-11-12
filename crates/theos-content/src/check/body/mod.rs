//! Body rules: P02's malformed constructs and the well-formed body data that break a rule,
//! judged per kind through its [`BodyGrammar`].
//!
//! Each rule is one module with its `BodyRule` constants; a new rule is one module plus one line
//! in [`BODY_RULES`], in the order of the phase spec's table (§5.3).

mod attribute_count;
mod attribute_duplicate;
mod attribute_unknown;
mod beat_number;
mod malformed;
mod note_id;
mod note_state;
mod tier;
mod when;

use crate::body::Body;
use crate::finding::Severity;
use crate::kinds::{BodyGrammar, Problem};
use crate::text::Span;

/// What a body rule reads: a parsed body and the grammar of its kind.
pub(crate) struct BodyInput<'a> {
    /// Its body.
    pub body: &'a Body,
    /// What a body of its kind may contain.
    pub grammar: &'a BodyGrammar,
}

/// A rule that judges a body.
pub(crate) struct BodyRule {
    /// `body/<name>`.
    pub id: &'static str,
    /// The default severity of its findings.
    pub severity: Severity,
    /// The rule.
    pub run: fn(&BodyInput<'_>) -> Vec<Problem>,
}

/// The body rules in the order of the spec's table.
pub(crate) const BODY_RULES: &[BodyRule] = &[
    malformed::LINK_UNCLOSED,
    malformed::LINK_TARGET,
    malformed::EMBED,
    malformed::CALLOUT_TITLE,
    malformed::BEAT_HEADING,
    malformed::BEAT_ATTRIBUTE,
    malformed::MISPLACED_ATTRIBUTES,
    tier::RULE,
    when::RULE,
    attribute_count::RULE,
    attribute_duplicate::RULE,
    attribute_unknown::RULE,
    beat_number::RULE,
    note_id::RULE,
    note_state::RULE,
];

/// A problem at `span`.
fn problem(span: Span, message: impl Into<String>) -> Problem {
    Problem {
        span,
        message: message.into(),
    }
}
