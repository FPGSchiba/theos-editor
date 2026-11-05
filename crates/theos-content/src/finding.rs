//! Findings: what a check reports about a record.

use std::fmt;

use crate::text::{Position, Span};

/// How serious a finding is by default.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Severity {
    /// The record is wrong.
    Error,
    /// The record is suspicious.
    Warning,
}

impl Severity {
    /// The lower-case name, as stored and printed.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Severity::Error => "error",
            Severity::Warning => "warning",
        }
    }

    /// Reads a name written by [`Severity::as_str`].
    #[must_use]
    pub fn parse(text: &str) -> Option<Severity> {
        match text {
            "error" => Some(Severity::Error),
            "warning" => Some(Severity::Warning),
            _ => None,
        }
    }
}

/// One thing a check found in a record.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Finding {
    /// The rule, `<group>/<name>`.
    pub rule: String,
    /// The default severity; P04 applies configured ones on read.
    pub severity: Severity,
    /// Project-relative path with `/`.
    pub path: String,
    /// The record's id (file stem); `None` for `record/path`.
    pub id: Option<String>,
    /// Record coordinates; `None` only for `record/path`.
    pub span: Option<Span>,
    /// Line and column of the span start.
    pub position: Option<Position>,
    /// What is wrong.
    pub message: String,
}

impl fmt::Display for Finding {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.path)?;
        if let Some(position) = self.position {
            write!(f, ":{}:{}", position.line, position.column)?;
        }
        write!(
            f,
            ": {} {}: {}",
            self.severity.as_str(),
            self.rule,
            self.message
        )
    }
}

/// `a` or `an` before `word`, for messages that name a kind; `npc` is read as letters, so it
/// takes `an`.
pub(crate) fn with_article(word: &str) -> String {
    let article = if word.starts_with(['a', 'e', 'i', 'o', 'u']) || word.starts_with("npc") {
        "an"
    } else {
        "a"
    };
    format!("{article} {word}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kinds_take_their_article() {
        for (kind, want) in [
            ("npc", "an npc"),
            ("equation", "an equation"),
            ("item", "an item"),
            ("thing", "a thing"),
            ("belief", "a belief"),
        ] {
            assert_eq!(with_article(kind), want);
        }
    }

    fn finding(path: &str, rule: &str, position: Option<Position>) -> Finding {
        Finding {
            rule: rule.to_owned(),
            severity: Severity::Error,
            path: path.to_owned(),
            id: None,
            span: None,
            position,
            message: "m".to_owned(),
        }
    }

    #[test]
    fn findings_display_with_and_without_position() {
        let at = Some(Position { line: 3, column: 7 });
        assert_eq!(
            finding("things/a.md", "schema/value", at).to_string(),
            "things/a.md:3:7: error schema/value: m"
        );
        assert_eq!(
            finding("x.md", "record/path", None).to_string(),
            "x.md: error record/path: m"
        );
    }

    #[test]
    fn severity_round_trips_and_rejects_unknown() {
        for severity in [Severity::Error, Severity::Warning] {
            assert_eq!(Severity::parse(severity.as_str()), Some(severity));
        }
        assert_eq!(Severity::parse("fatal"), None);
    }
}
