//! `body/note-state`: a note is `open`, `resolved by [[id]]` or `dropped` (CE-15.5).

use super::{BodyInput, BodyRule, problem};
use crate::body::{CalloutTitle, NoteState};
use crate::finding::Severity;
use crate::kinds::Problem;

/// The rule.
pub(super) const RULE: BodyRule = BodyRule {
    id: "body/note-state",
    severity: Severity::Error,
    run,
};

fn run(input: &BodyInput<'_>) -> Vec<Problem> {
    if input.grammar.notes.is_none() {
        return Vec::new();
    }
    input
        .body
        .callouts()
        .iter()
        .filter_map(|callout| match &callout.title {
            Some(CalloutTitle::Note(note)) => Some(note),
            _ => None,
        })
        .filter_map(|note| match &note.state {
            NoteState::Unknown(word) => Some(problem(
                note.state_span,
                format!("`{word}` is not a note state; use open, resolved by [[id]] or dropped"),
            )),
            _ => None,
        })
        .collect()
}
