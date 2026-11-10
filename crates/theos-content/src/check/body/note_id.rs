//! `body/note-id`: the notes of a record have different ids.

use super::{BodyInput, BodyRule, problem};
use crate::body::CalloutTitle;
use crate::finding::Severity;
use crate::kinds::Problem;

/// The rule.
pub(super) const RULE: BodyRule = BodyRule {
    id: "body/note-id",
    severity: Severity::Error,
    run,
};

fn run(input: &BodyInput<'_>) -> Vec<Problem> {
    if input.grammar.notes.is_none() {
        return Vec::new();
    }
    let ids: Vec<_> = input
        .body
        .callouts()
        .iter()
        .filter_map(|callout| match &callout.title {
            Some(CalloutTitle::Note(note)) => Some(&note.id),
            _ => None,
        })
        .collect();
    ids.iter()
        .enumerate()
        .filter(|(i, id)| ids[..*i].iter().any(|earlier| earlier.text == id.text))
        .map(|(_, id)| problem(id.span, format!("the note id `{}` is used twice", id.text)))
        .collect()
}
