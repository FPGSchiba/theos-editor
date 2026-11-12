//! The errors a command introduces: the shared verdict of the commands and the CLI.

use std::collections::{BTreeMap, BTreeSet};

use theos_content::{Finding, Severity};

/// Findings counted per (path, rule), with the messages seen.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FindingCounts {
    counts: BTreeMap<(String, String), usize>,
    messages: BTreeSet<(String, String, String)>,
}

impl FindingCounts {
    /// Counts `findings` of every severity; `moved` maps old path to new path.
    #[must_use]
    pub fn of(findings: &[Finding], moved: &BTreeMap<String, String>) -> FindingCounts {
        let mut counts = FindingCounts::default();
        for finding in findings {
            let path = moved.get(&finding.path).unwrap_or(&finding.path);
            *counts
                .counts
                .entry((path.clone(), finding.rule.clone()))
                .or_default() += 1;
            counts
                .messages
                .insert((path.clone(), finding.rule.clone(), finding.message.clone()));
        }
        counts
    }

    /// The larger count per key; the messages united.
    #[must_use]
    pub fn max(&self, other: &FindingCounts) -> FindingCounts {
        let mut counts = self.counts.clone();
        for (key, count) in &other.counts {
            let entry = counts.entry(key.clone()).or_default();
            *entry = (*entry).max(*count);
        }
        FindingCounts {
            counts,
            messages: self.messages.union(&other.messages).cloned().collect(),
        }
    }

    /// How many findings of `rule` on `path` were counted.
    #[must_use]
    pub fn count(&self, path: &str, rule: &str) -> usize {
        self.counts
            .get(&(path.to_owned(), rule.to_owned()))
            .copied()
            .unwrap_or(0)
    }

    fn has_seen(&self, finding: &Finding) -> bool {
        self.messages.contains(&(
            finding.path.clone(),
            finding.rule.clone(),
            finding.message.clone(),
        ))
    }
}

/// The errors of `after` above `allowed`'s count for their (path, rule), in `after` order. Where
/// a key has more errors than allowed, the surplus is taken first from the errors whose message
/// `allowed` has not seen, then from the rest. Warnings are never introduced.
#[must_use]
pub fn introduced(allowed: &FindingCounts, after: &[Finding]) -> Vec<Finding> {
    let mut by_key: BTreeMap<(&str, &str), Vec<usize>> = BTreeMap::new();
    for (position, finding) in after.iter().enumerate() {
        if finding.severity == Severity::Error {
            by_key
                .entry((&finding.path, &finding.rule))
                .or_default()
                .push(position);
        }
    }
    let mut taken = BTreeSet::new();
    for ((path, rule), positions) in by_key {
        let surplus = positions.len().saturating_sub(allowed.count(path, rule));
        let (unseen, seen): (Vec<usize>, Vec<usize>) = positions
            .into_iter()
            .partition(|&position| !allowed.has_seen(&after[position]));
        taken.extend(unseen.into_iter().chain(seen).take(surplus));
    }
    taken.into_iter().map(|p| after[p].clone()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn finding(path: &str, rule: &str, severity: Severity, message: &str) -> Finding {
        Finding {
            rule: rule.to_owned(),
            severity,
            path: path.to_owned(),
            id: None,
            span: None,
            position: None,
            message: message.to_owned(),
        }
    }

    fn error(path: &str, rule: &str, message: &str) -> Finding {
        finding(path, rule, Severity::Error, message)
    }

    fn none() -> FindingCounts {
        FindingCounts::of(&[], &BTreeMap::new())
    }

    const RULE: &str = "clue/moves-truth";

    #[test]
    // AC: CE-20.1
    fn a_new_error_of_a_rule_on_a_path_is_introduced() {
        let after = [error("a.md", RULE, "No clue moves T-981")];
        assert_eq!(introduced(&none(), &after), after);
    }

    #[test]
    // AC: CE-20.1
    fn a_changed_message_of_a_standing_error_is_not_introduced() {
        let before = [error("a.md", RULE, "No clue moves T-981")];
        let allowed = FindingCounts::of(&before, &BTreeMap::new());
        let after = [error("a.md", RULE, "Only one clue moves T-981")];
        assert_eq!(introduced(&allowed, &after), Vec::<Finding>::new());
    }

    #[test]
    // AC: CE-20.1
    fn a_raised_severity_is_not_introduced() {
        let before = [finding(
            "a.md",
            RULE,
            Severity::Warning,
            "No clue moves T-981",
        )];
        let allowed = FindingCounts::of(&before, &BTreeMap::new());
        let after = [error("a.md", RULE, "No clue moves T-981")];
        assert_eq!(introduced(&allowed, &after), Vec::<Finding>::new());
    }

    #[test]
    // AC: CE-20.1
    fn warnings_are_never_introduced() {
        let after = [finding(
            "a.md",
            RULE,
            Severity::Warning,
            "No clue moves T-981",
        )];
        assert_eq!(introduced(&none(), &after), Vec::<Finding>::new());
    }

    #[test]
    // AC: CE-20.1
    fn a_moved_file_keeps_its_findings() {
        let before = [
            error("old.md", RULE, "No clue moves T-981"),
            error("keep.md", RULE, "No clue moves T-982"),
        ];
        let moved = BTreeMap::from([("old.md".to_owned(), "new.md".to_owned())]);
        let allowed = FindingCounts::of(&before, &moved);
        assert_eq!(allowed.count("new.md", RULE), 1);
        assert_eq!(allowed.count("old.md", RULE), 0);
        assert_eq!(allowed.count("keep.md", RULE), 1);
        let after = [
            error("new.md", RULE, "No clue moves T-981"),
            error("keep.md", RULE, "No clue moves T-982"),
        ];
        assert_eq!(introduced(&allowed, &after), Vec::<Finding>::new());
    }

    #[test]
    // AC: CE-20.1
    fn max_takes_the_larger_count_per_key() {
        let one = FindingCounts::of(&[error("a.md", RULE, "x")], &BTreeMap::new());
        let two = FindingCounts::of(
            &[error("a.md", RULE, "y"), error("a.md", RULE, "z")],
            &BTreeMap::new(),
        );
        let max = one.max(&two);
        assert_eq!(max.count("a.md", RULE), 2);
        assert_eq!(two.max(&one).count("a.md", RULE), 2);
        assert!(max.has_seen(&error("a.md", RULE, "x")));
        assert!(max.has_seen(&error("a.md", RULE, "z")));
    }

    #[test]
    // AC: CE-20.1
    fn max_keeps_the_keys_of_both_sides() {
        let a = FindingCounts::of(&[error("a.md", RULE, "x")], &BTreeMap::new());
        let b = FindingCounts::of(&[error("b.md", RULE, "y")], &BTreeMap::new());
        let max = a.max(&b);
        assert_eq!(max.count("a.md", RULE), 1);
        assert_eq!(max.count("b.md", RULE), 1);
        assert!(max.has_seen(&error("a.md", RULE, "x")));
        assert!(max.has_seen(&error("b.md", RULE, "y")));
    }

    #[test]
    // AC: CE-20.1
    fn unseen_messages_come_first() {
        let allowed = FindingCounts::of(&[error("a.md", RULE, "seen")], &BTreeMap::new());
        let after = [error("a.md", RULE, "seen"), error("a.md", RULE, "unseen")];
        assert_eq!(introduced(&allowed, &after), [after[1].clone()]);
    }

    #[test]
    // AC: CE-20.1
    fn unseen_messages_win_over_a_seen_one_that_comes_first() {
        let allowed = FindingCounts::of(&[error("a.md", RULE, "seen")], &BTreeMap::new());
        let after = [
            error("a.md", RULE, "seen"),
            error("a.md", RULE, "new 1"),
            error("a.md", RULE, "new 2"),
        ];
        assert_eq!(
            introduced(&allowed, &after),
            [after[1].clone(), after[2].clone()]
        );
    }

    #[test]
    // AC: CE-20.1
    fn a_surplus_beyond_the_unseen_takes_seen_ones_and_keeps_after_order() {
        let allowed = FindingCounts::of(&[error("a.md", RULE, "seen")], &BTreeMap::new());
        let after = [
            error("a.md", RULE, "seen"),
            error("a.md", RULE, "seen"),
            error("a.md", RULE, "new"),
        ];
        // Surplus 2: the unseen one first, then the first seen one; returned in `after` order.
        assert_eq!(
            introduced(&allowed, &after),
            [after[0].clone(), after[2].clone()]
        );
    }

    #[test]
    // AC: CE-20.1
    fn count_is_zero_for_an_unknown_key() {
        let counts = FindingCounts::of(&[error("a.md", RULE, "x")], &BTreeMap::new());
        assert_eq!(counts.count("a.md", RULE), 1);
        assert_eq!(counts.count("a.md", "other/rule"), 0);
        assert_eq!(counts.count("b.md", RULE), 0);
    }
}
