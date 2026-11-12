//! The declarative description of one kind.

use std::fmt;

use super::types::{FieldType, LENS_ENTRIES, Target};
use crate::text::Span;
use crate::yaml::{FrontMatter, Order};

/// One front-matter field of a kind.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct FieldSpec {
    /// The field key.
    pub key: &'static str,
    /// Its type.
    pub ty: FieldType,
    /// Whether the record must have it.
    pub required: bool,
}

impl fmt::Display for FieldSpec {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.key, self.ty)?;
        if self.required {
            f.write_str(", required")?;
        }
        Ok(())
    }
}

/// The pattern an id of a kind must match, and what to say about it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct IdRule {
    /// A regular expression; the author anchors it.
    pub pattern: &'static str,
    /// Plain words for the message when the id does not fit.
    pub says: &'static str,
}

/// Where the records of a kind live.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Placement {
    /// Flat in a folder.
    Flat(&'static str),
    /// Under the folder of the record the named parent field links to.
    UnderParent(&'static str),
}

/// A beat attribute and the kinds it may link to.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct BeatAttribute {
    /// The attribute key.
    pub key: &'static str,
    /// Its link targets.
    pub targets: &'static [Target],
}

/// What a body of a kind may contain.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct BodyGrammar {
    /// The beat attributes that hold links, when the body has beats.
    pub beats: Option<&'static [BeatAttribute]>,
    /// The beat attributes that are one bare word, such as `when`.
    pub beat_tokens: &'static [&'static str],
    /// The kinds a note may link to when the body has notes.
    pub notes: Option<&'static [Target]>,
    /// Whether the body is the page's text as it is, not Markdown to judge.
    pub verbatim: bool,
}

impl BodyGrammar {
    /// A body of prose: no beats, no notes.
    pub const PROSE: BodyGrammar = BodyGrammar {
        beats: None,
        beat_tokens: &[],
        notes: None,
        verbatim: false,
    };

    /// A body that is verbatim text: no links, no beats, no notes, and no body rule judges it.
    pub const VERBATIM: BodyGrammar = BodyGrammar {
        verbatim: true,
        ..BodyGrammar::PROSE
    };
}

/// A problem a kind check found.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Problem {
    /// Where in the record.
    pub span: Span,
    /// What is wrong.
    pub message: String,
}

/// What a kind check reads.
pub struct CheckInput<'a> {
    /// The record id.
    pub id: &'a str,
    /// The front matter.
    pub front: &'a FrontMatter,
    /// The span of the `kind` field value.
    pub kind_span: Span,
}

/// A rule only one kind has.
#[derive(Clone, Copy)]
pub struct KindCheck {
    /// The rule, `<group>/<name>`.
    pub rule: &'static str,
    /// The check.
    pub run: fn(&CheckInput<'_>) -> Vec<Problem>,
}

impl fmt::Debug for KindCheck {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // A fn pointer's address differs per run; the fingerprint is built from this output.
        f.debug_struct("KindCheck")
            .field("rule", &self.rule)
            .finish_non_exhaustive()
    }
}

/// The complete description of a kind.
#[derive(Clone, Debug)]
pub struct KindSpec {
    /// The kind name.
    pub name: &'static str,
    /// The id rule.
    pub id: IdRule,
    /// Where records live.
    pub placement: Placement,
    /// The fields, in writing order.
    pub fields: Vec<FieldSpec>,
    /// The body grammar.
    pub body: BodyGrammar,
    /// Checks only this kind has.
    pub checks: Vec<KindCheck>,
}

impl KindSpec {
    /// The field `key`, if the kind has it.
    #[must_use]
    pub fn field(&self, key: &str) -> Option<&FieldSpec> {
        self.fields.iter().find(|field| field.key == key)
    }

    /// The writing order of the fields, with the lens entries when a field is a lens.
    #[must_use]
    pub fn order(&self) -> Order {
        let order = Order::new(self.fields.iter().map(|field| field.key));
        if self.fields.iter().any(|field| field.ty == FieldType::Lens) {
            order.with_entries("lens", LENS_ENTRIES.iter().map(|entry| entry.key))
        } else {
            order
        }
    }
}
