//! The undo journal of a workspace (spec §5.7): the commands it applied, and those undone.
//!
//! The journal lives in memory only and ends with the workspace.

use std::collections::VecDeque;

use super::route::Applied;
use crate::Workspace;

/// The id of a journal entry, unique within its workspace.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct EntryId(pub(crate) u64);

/// What the undo and redo lists show of a journal entry.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EntrySummary {
    /// The entry.
    pub id: EntryId,
    /// The label of its command.
    pub label: String,
    /// The paths its command changed, in plan order.
    pub paths: Vec<String>,
}

/// One applied command.
#[derive(Debug)]
struct Entry {
    id: EntryId,
    applied: Applied,
}

impl Entry {
    fn summary(&self) -> EntrySummary {
        EntrySummary {
            id: self.id,
            label: self.applied.label.clone(),
            paths: self
                .applied
                .changes
                .iter()
                .map(|change| change.path.clone())
                .collect(),
        }
    }
}

/// The two lists of a workspace: done (oldest first) and undone (most recently undone last).
#[derive(Debug, Default)]
pub(crate) struct Journal {
    done: VecDeque<Entry>,
    undone: Vec<Entry>,
    next: u64,
}

impl Journal {
    /// How many entries done keeps; older ones are dropped.
    pub(crate) const LIMIT: usize = 100;

    /// Adds `applied` as the newest entry of done and clears undone.
    pub(crate) fn push(&mut self, applied: Applied) -> EntryId {
        let id = EntryId(self.next);
        self.next += 1;
        self.done.push_back(Entry { id, applied });
        while self.done.len() > Self::LIMIT {
            self.done.pop_front();
        }
        self.undone.clear();
        id
    }
}

impl Workspace {
    /// The entries undo can take back, newest first.
    #[must_use]
    pub fn undo_entries(&self) -> Vec<EntrySummary> {
        let done = &self.commands.journal.done;
        done.iter().rev().map(Entry::summary).collect()
    }

    /// The entries redo can apply again, the most recently undone first.
    #[must_use]
    pub fn redo_entries(&self) -> Vec<EntrySummary> {
        let undone = &self.commands.journal.undone;
        undone.iter().rev().map(Entry::summary).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::FindingCounts;
    use crate::command::FileChange;

    fn applied(label: &str) -> Applied {
        Applied {
            label: label.to_owned(),
            changes: vec![FileChange {
                path: format!("{label}.md"),
                before: None,
                after: Some(b"x".to_vec()),
            }],
            counts_before: FindingCounts::default(),
            counts_after: FindingCounts::default(),
        }
    }

    fn labels(journal: &Journal) -> Vec<String> {
        journal
            .done
            .iter()
            .rev()
            .map(|entry| entry.summary().label)
            .collect()
    }

    #[test]
    fn push_gives_new_ids_and_lists_newest_first() {
        let mut journal = Journal::default();
        let a = journal.push(applied("a"));
        let b = journal.push(applied("b"));
        assert!(a < b);
        assert_eq!(labels(&journal), ["b", "a"]);
        let summary = journal.done.back().map(Entry::summary);
        assert_eq!(
            summary,
            Some(EntrySummary {
                id: b,
                label: "b".to_owned(),
                paths: vec!["b.md".to_owned()],
            })
        );
    }

    #[test]
    fn push_clears_undone() {
        let mut journal = Journal::default();
        journal.push(applied("a"));
        let undone = journal.done.pop_back();
        journal.undone.extend(undone);
        assert_eq!(journal.undone.len(), 1);
        journal.push(applied("b"));
        assert!(journal.undone.is_empty());
    }

    #[test]
    fn done_keeps_the_newest_entries() {
        let mut journal = Journal::default();
        for n in 0..=Journal::LIMIT {
            journal.push(applied(&n.to_string()));
        }
        assert_eq!(journal.done.len(), Journal::LIMIT);
        let newest = Journal::LIMIT.to_string();
        assert_eq!(labels(&journal).first(), Some(&newest));
        assert_eq!(labels(&journal).last().map(String::as_str), Some("1"));
    }
}
