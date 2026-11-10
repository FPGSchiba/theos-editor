//! Checks across records (spec §5.5): lore rules, the subjects they check and the [`Context`]
//! they read through. A rule gets data only from its context, which stores every read, so a
//! change re-runs exactly the subjects that read something it touched.
//!
//! A part registers its rules with [`KindRegistry::register_rule`]; the index runs them after it
//! has resolved the links, and stores their findings with the step `lore`.
//!
//! [`KindRegistry::register_rule`]: crate::kinds::KindRegistry::register_rule

mod context;
mod facts;

#[cfg_attr(not(test), expect(dead_code, reason = "wired in by Task 8"))]
pub(crate) mod availability;
pub(crate) mod cycle;
#[cfg_attr(not(test), expect(dead_code, reason = "wired in by Task 11"))]
pub(crate) mod lens;
#[cfg_attr(not(test), expect(dead_code, reason = "wired in by Task 8"))]
pub(crate) mod places;

use std::fmt;
use std::rc::Rc;

pub use context::{Context, FactError};
pub(crate) use context::{FactSource, ReadKey};
pub use facts::{
    FactBeat, FactLink, FactNote, FactValue, InLink, InputFacts, RecordFacts, Segment,
};

use crate::finding::{Finding, Severity};
use crate::rules::RuleClass;
use crate::text::{Position, Span};

/// The subjects a lore rule checks.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum On {
    /// The records of these kinds.
    Kinds(&'static [&'static str]),
    /// Every record, also one whose kind is unknown.
    AnyRecord,
    /// The input file at this project-relative path.
    Input(&'static str),
}

impl On {
    /// Whether a rule on `self` checks a record of the kind `kind`.
    pub(crate) fn takes_record(self, kind: Option<&str>) -> bool {
        match self {
            On::Kinds(kinds) => kind.is_some_and(|kind| kinds.contains(&kind)),
            On::AnyRecord => true,
            On::Input(_) => false,
        }
    }

    /// Whether a rule on `self` and one on `other` check a common subject: a kind in both
    /// lists, `AnyRecord` next to any record rule, or the same input path.
    pub(crate) fn overlaps(self, other: On) -> bool {
        match (self, other) {
            (On::Kinds(a), On::Kinds(b)) => a.iter().any(|kind| b.contains(kind)),
            (On::AnyRecord, On::AnyRecord | On::Kinds(_)) | (On::Kinds(_), On::AnyRecord) => true,
            (On::Input(a), On::Input(b)) => a == b,
            _ => false,
        }
    }

    /// Whether a rule on `self` checks the input file at `path`.
    pub(crate) fn takes_input(self, path: &str) -> bool {
        matches!(self, On::Input(input) if input == path)
    }
}

/// A check across records: it reads its subject and whatever else it needs through the
/// [`Context`], and returns findings on its subject only, made with [`Subject::finding`].
#[derive(Clone, Copy)]
pub struct LoreRule {
    /// The rule, `<group>/<name>`.
    pub id: &'static str,
    /// The severity when nothing configures it.
    pub severity: Severity,
    /// Whether and how settings may change it.
    pub class: RuleClass,
    /// The subjects it checks.
    pub on: On,
    /// The check. It fails only when the store fails, never on content (L5).
    pub run: fn(&mut Context<'_>, &Subject) -> Result<Vec<Finding>, FactError>,
}

impl fmt::Debug for LoreRule {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // A fn pointer's address differs per run; the registry fingerprint is built from this.
        f.debug_struct("LoreRule")
            .field("id", &self.id)
            .field("severity", &self.severity)
            .field("class", &self.class)
            .field("on", &self.on)
            .finish_non_exhaustive()
    }
}

/// What the lore step checks: a record, or an input file of the project.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Subject {
    /// A record, with its facts.
    Record(Rc<RecordFacts>),
    /// An input file, such as `.theos/settings.toml`.
    Input(Rc<InputFacts>),
}

impl Subject {
    /// The project-relative path of the subject.
    #[must_use]
    pub fn path(&self) -> &str {
        match self {
            Subject::Record(record) => &record.path,
            Subject::Input(input) => &input.path,
        }
    }

    /// A finding of `rule` on this subject: the rule's id and severity, the subject's path, and
    /// its id for a record (none for an input file, L9).
    #[must_use]
    pub fn finding(
        &self,
        rule: &LoreRule,
        span: Span,
        position: Position,
        message: String,
    ) -> Finding {
        Finding {
            rule: rule.id.to_owned(),
            severity: rule.severity,
            path: self.path().to_owned(),
            id: match self {
                Subject::Record(record) => Some(record.id.clone()),
                Subject::Input(_) => None,
            },
            span: Some(span),
            position: Some(position),
            message,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const RULE: LoreRule = LoreRule {
        id: "test/rule",
        severity: Severity::Warning,
        class: RuleClass::Configurable,
        on: On::Kinds(&["truth", "clue"]),
        run: |_, _| Ok(Vec::new()),
    };

    fn record() -> Subject {
        Subject::Record(Rc::new(RecordFacts {
            path: "lore/truths/T-950.md".to_owned(),
            id: "T-950".to_owned(),
            kind: Some("truth".to_owned()),
            hash: String::new(),
            values: Vec::new(),
            links: Vec::new(),
            beats: Vec::new(),
            notes: Vec::new(),
            inworld: Vec::new(),
        }))
    }

    fn input() -> Subject {
        Subject::Input(Rc::new(InputFacts {
            path: ".theos/settings.toml".to_owned(),
            hash: String::new(),
            text: None,
        }))
    }

    #[test]
    fn rules_check_the_subjects_they_are_on() {
        let kinds = On::Kinds(&["truth", "clue"]);
        assert!(kinds.takes_record(Some("clue")));
        assert!(!kinds.takes_record(Some("thing")));
        assert!(!kinds.takes_record(None));
        assert!(!kinds.takes_input("lore/truths/T-950.md"));
        assert!(On::AnyRecord.takes_record(None));
        assert!(On::AnyRecord.takes_record(Some("thing")));
        assert!(!On::AnyRecord.takes_input(".theos/settings.toml"));
        let settings = On::Input(".theos/settings.toml");
        assert!(settings.takes_input(".theos/settings.toml"));
        assert!(!settings.takes_input("canon/formalism/_source.json"));
        assert!(!settings.takes_record(None));
    }

    #[test]
    fn a_finding_sits_on_its_subject() {
        let at = Position { line: 3, column: 5 };
        let span = Span { start: 7, end: 9 };
        let on_record = record().finding(&RULE, span, at, "m".to_owned());
        assert_eq!(
            on_record.to_string(),
            "lore/truths/T-950.md:3:5: warning test/rule: m"
        );
        assert_eq!(on_record.id.as_deref(), Some("T-950"));
        assert_eq!(on_record.span, Some(span));
        let on_input = input().finding(&RULE, span, at, "m".to_owned());
        assert_eq!(on_input.path, ".theos/settings.toml");
        assert_eq!(on_input.id, None, "a finding on an input file has no id");
        assert_eq!(input().path(), ".theos/settings.toml");
    }

    #[test]
    fn a_rule_shows_without_its_function() {
        assert_eq!(
            format!("{RULE:?}"),
            "LoreRule { id: \"test/rule\", severity: Warning, class: Configurable, \
             on: Kinds([\"truth\", \"clue\"]), .. }"
        );
    }
}
