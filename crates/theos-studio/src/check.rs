//! Check: a project's findings, plus the binaries-folder warnings when a folder is known.

use std::path::PathBuf;

use theos_content::{Finding, Severity};

use crate::workspace::Workspace;
use crate::{StudioError, report, user_settings};

/// Why binaries are not checked when the user settings name no folder.
const NOT_SET: &str = "no binaries folder set";

/// Where the binaries folder comes from, resolved before the check runs.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BinariesFolder {
    /// The folder to check against, as given (a relative path is used as written).
    Folder(PathBuf),
    /// No flag and no setting.
    NotSet,
    /// The settings could not be read; the text says why.
    Unreadable(String),
}

/// The folder given by `flag`, else the one the user settings name.
#[must_use]
pub fn binaries_folder(flag: Option<PathBuf>) -> BinariesFolder {
    if let Some(folder) = flag {
        return BinariesFolder::Folder(folder);
    }
    match user_settings::binaries_root() {
        Ok(Some(folder)) => BinariesFolder::Folder(folder),
        Ok(None) => BinariesFolder::NotSet,
        Err(error) => BinariesFolder::Unreadable(report(&error)),
    }
}

/// Whether the binaries were checked.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Binaries {
    /// They were, against this folder.
    Checked {
        /// The folder.
        folder: PathBuf,
    },
    /// They were not.
    NotChecked {
        /// Why not.
        reason: String,
    },
}

/// The result of checking a project.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Check {
    /// The findings of the index and, when a folder is known, those of its binaries, in
    /// the order of [`order`].
    pub findings: Vec<Finding>,
    /// Whether the binaries were checked, and why not.
    pub binaries: Binaries,
}

/// Checks the project: its findings, and the binary warnings when `binaries` names a folder.
///
/// # Errors
///
/// [`StudioError::Index`] when the index fails.
pub fn check(workspace: &mut Workspace, binaries: BinariesFolder) -> Result<Check, StudioError> {
    let mut findings = workspace.index().findings()?;
    let binaries = match binaries {
        BinariesFolder::Folder(folder) => {
            let report = workspace.write(|index| index.binary_findings(&folder))?;
            findings.extend(report.findings);
            order(&mut findings);
            Binaries::Checked { folder }
        }
        BinariesFolder::NotSet => Binaries::NotChecked {
            reason: NOT_SET.to_owned(),
        },
        BinariesFolder::Unreadable(reason) => Binaries::NotChecked { reason },
    };
    Ok(Check { findings, binaries })
}

/// Sorts findings as the index does: by path, then position (none first), rule, message and
/// span start.
pub fn order(findings: &mut [Finding]) {
    findings.sort_by(|a, b| {
        (
            &a.path,
            a.position,
            &a.rule,
            &a.message,
            a.span.map(|s| s.start),
        )
            .cmp(&(
                &b.path,
                b.position,
                &b.rule,
                &b.message,
                b.span.map(|s| s.start),
            ))
    });
}

/// The number of errors and of warnings among `findings`.
#[must_use]
pub fn counts(findings: &[Finding]) -> (usize, usize) {
    let count = |severity| findings.iter().filter(|f| f.severity == severity).count();
    (count(Severity::Error), count(Severity::Warning))
}

#[cfg(test)]
mod tests {
    use theos_content::{Position, Span};

    use super::*;
    use crate::test_support::{error, warning};

    fn at(finding: Finding, line: usize, start: usize) -> Finding {
        Finding {
            position: Some(Position { line, column: 1 }),
            span: Some(Span {
                start,
                end: start + 1,
            }),
            ..finding
        }
    }

    #[test]
    fn a_finding_without_a_position_comes_before_one_with_a_position() {
        let mut findings = vec![
            at(warning("a.md", "x/rule", "m"), 1, 0),
            warning("a.md", "x/rule", "m"),
        ];
        order(&mut findings);
        assert_eq!(findings[0].position, None);
        assert!(findings[1].position.is_some());
    }

    #[test]
    fn findings_at_one_position_are_ordered_by_rule_then_message_then_span() {
        let mut findings = vec![
            at(warning("a.md", "b/rule", "m"), 1, 0),
            at(warning("a.md", "a/rule", "z"), 1, 0),
            at(warning("a.md", "a/rule", "m"), 1, 5),
            at(warning("a.md", "a/rule", "m"), 1, 2),
        ];
        order(&mut findings);
        let keys: Vec<_> = findings
            .iter()
            .map(|f| (f.rule.as_str(), f.message.as_str(), f.span.map(|s| s.start)))
            .collect();
        assert_eq!(
            keys,
            [
                ("a/rule", "m", Some(2)),
                ("a/rule", "m", Some(5)),
                ("a/rule", "z", Some(0)),
                ("b/rule", "m", Some(0)),
            ]
        );
    }

    #[test]
    fn the_path_decides_before_the_position() {
        let mut findings = vec![
            warning("b.md", "x/rule", "m"),
            at(warning("a.md", "x/rule", "m"), 9, 0),
        ];
        order(&mut findings);
        assert_eq!(findings[0].path, "a.md");
    }

    #[test]
    fn counts_split_errors_from_warnings() {
        let findings = [
            error("a.md", "x/a", "m"),
            warning("a.md", "x/b", "m"),
            warning("a.md", "x/c", "m"),
        ];
        assert_eq!(counts(&findings), (1, 2));
    }
}
