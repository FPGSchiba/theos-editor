//! The verdict (spec §5.5, CE-20.1): a command that raises the number of errors of a rule on a
//! path is rejected. Standing errors and warnings do not block; the new warnings are reported. A
//! command that would take a record it moves out of the index is refused (ruling R16).

use std::collections::BTreeMap;

use theos_content::{Finding, Severity, Update};

use super::CommandError;
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

/// Ruling R16: a command never takes a record out of the index. Fails with
/// [`CommandError::NotARecord`] for the first file of `moved` (old path to new path) that was a
/// record at its old path, so the preview's `update` removed it there, and that the update did
/// not read as a record at its new path: hidden, ignored by git, or not a record name.
pub(crate) fn keep_records(
    moved: &BTreeMap<String, String>,
    update: &Update,
) -> Result<(), CommandError> {
    let lost = moved
        .iter()
        .find(|(old, new)| update.removed.contains(old) && !update.reparsed.contains(new));
    match lost {
        Some((_, new)) => Err(CommandError::NotARecord { path: new.clone() }),
        None => Ok(()),
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

    fn update(removed: &[&str], reparsed: &[&str]) -> Update {
        let owned = |paths: &[&str]| paths.iter().map(|path| (*path).to_owned()).collect();
        Update {
            removed: owned(removed),
            reparsed: owned(reparsed),
            ..Update::default()
        }
    }

    /// Ruling R16: a moved record that the preview reads at its new path keeps its place.
    #[test]
    fn a_moved_record_read_at_its_new_path_is_kept() {
        let moved = BTreeMap::from([("a/old.md".to_owned(), "a/new.md".to_owned())]);
        let result = keep_records(&moved, &update(&["a/old.md"], &["a/new.md"]));
        assert!(result.is_ok(), "{result:?}");
    }

    /// Ruling R16: a moved record that the preview does not read at its new path is refused.
    #[test]
    fn a_moved_record_not_read_at_its_new_path_is_refused() {
        let moved = BTreeMap::from([
            ("a/keep.md".to_owned(), "b/keep.md".to_owned()),
            ("a/old.md".to_owned(), "a/README.md".to_owned()),
        ]);
        let update = update(&["a/keep.md", "a/old.md"], &["b/keep.md"]);
        let result = keep_records(&moved, &update);
        assert!(
            matches!(&result, Err(CommandError::NotARecord { path }) if path == "a/README.md"),
            "{result:?}"
        );
    }

    /// A moved file that was no record before (notes, a hidden file) is no record after either.
    #[test]
    fn a_moved_file_that_was_no_record_is_left_alone() {
        let moved = BTreeMap::from([
            ("a/notes.txt".to_owned(), "b/notes.txt".to_owned()),
            ("a/.x.md".to_owned(), "b/.x.md".to_owned()),
        ]);
        let result = keep_records(&moved, &update(&[], &[]));
        assert!(result.is_ok(), "{result:?}");
    }
}
