//! The verdict (spec §5.5, CE-20.1): a command that raises the number of errors of a rule on a
//! path is rejected. Standing errors and warnings do not block; the new warnings are reported.

use std::collections::BTreeMap;

use theos_content::{Finding, Severity};

use crate::introduced::{FindingCounts, introduced, surplus};

/// What the verdict found in a preview.
#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct Verdict {
    /// The new errors; the command is rejected when there is one.
    pub(crate) blocking: Vec<Finding>,
    /// The new warnings.
    pub(crate) introduced: Vec<Finding>,
}

/// Judges the findings `after` a command against the findings `before` it on the same paths.
/// `moved` maps the old path of each file the command moves to its new one. A replay of the
/// journal brings an `allowance`: the counts of the state it restores, which are allowed too.
pub(crate) fn judge(
    before: &[Finding],
    moved: &BTreeMap<String, String>,
    allowance: Option<&FindingCounts>,
    after: &[Finding],
) -> Verdict {
    let now = FindingCounts::of(before, moved);
    let allowed = match allowance {
        Some(allowance) => now.max(allowance),
        None => now,
    };
    Verdict {
        blocking: introduced(&allowed, after),
        introduced: surplus(&allowed, after, Severity::Warning),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{error, warning};

    const DANGLING: &str = "link/dangling";
    const THREE: &str = "lore/three-clue";

    // AC: CE-20.1
    #[test]
    fn a_new_error_blocks_and_a_standing_one_does_not() {
        let before = [error("a.md", DANGLING, "an error")];
        let after = [
            error("a.md", DANGLING, "an error"),
            error("b.md", DANGLING, "an error"),
        ];
        let verdict = judge(&before, &BTreeMap::new(), None, &after);
        assert_eq!(verdict.blocking, [after[1].clone()]);
        assert_eq!(verdict.introduced, Vec::<Finding>::new());
    }

    // AC: CE-20.1
    #[test]
    fn warnings_above_the_count_before_are_new_and_never_block() {
        let before = [warning("a.md", THREE, "No clue moves T-960")];
        let after = [
            warning("a.md", THREE, "Only one clue moves T-960"),
            warning("b.md", THREE, "No clue moves T-962"),
        ];
        let verdict = judge(&before, &BTreeMap::new(), None, &after);
        assert_eq!(verdict.blocking, Vec::<Finding>::new());
        assert_eq!(verdict.introduced, [after[1].clone()]);
    }

    // AC: CE-20.1
    #[test]
    fn a_moved_file_keeps_its_findings_under_its_new_path() {
        let before = [error("old.md", DANGLING, "an error")];
        let moved = BTreeMap::from([("old.md".to_owned(), "new.md".to_owned())]);
        let after = [error("new.md", DANGLING, "an error")];
        assert_eq!(judge(&before, &moved, None, &after), Verdict::default());
    }

    #[test]
    fn an_allowance_lets_the_restored_errors_back() {
        let allowance = FindingCounts::of(&[error("a.md", DANGLING, "an error")], &BTreeMap::new());
        let after = [
            error("a.md", DANGLING, "an error"),
            error("b.md", DANGLING, "an error"),
        ];
        let verdict = judge(&[], &BTreeMap::new(), Some(&allowance), &after);
        assert_eq!(verdict.blocking, [after[1].clone()]);
    }
}
