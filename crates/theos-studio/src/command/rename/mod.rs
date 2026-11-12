//! The command that renames an id (spec §5.8, CE-20.3): the record's file name and `id` field,
//! its child folder, a concept's or map's versions, and every link the index knows to any of
//! them, in one plan. What a rename covers is built by its participants (`participant`).

mod children;
mod ids;
mod links;
mod participant;
mod versions;

use std::collections::BTreeSet;

use theos_content::rename::{IdMap, edits};

use super::edit::parse;
use super::{Command, CommandError, FileChange, Plan, PlanContext, Reads, project_path};
use participant::{PARTICIPANTS, RenameSet, folder_of, join, on_disk, record};

/// Renames the record `id` to `to`: its file, its `id` field, its child folder with every file in
/// it, the versions of a concept or map, and every link to one of these ids that the index knows.
/// Whether `to` fits the kind's id rule is left to the verdict, and so is whether each moved
/// record is still read as a record at its new path ([`CommandError::NotARecord`]). A child
/// folder that holds a `.gitignore` is not moved: the rename fails with [`CommandError::Path`]
/// (ruling R21).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Rename {
    /// The id to rename.
    pub id: String,
    /// The new id.
    pub to: String,
}

impl Command for Rename {
    fn label(&self) -> String {
        format!("rename {} to {}", self.id, self.to)
    }

    /// A rename reads every record that links to the id, also those changed outside the editor.
    fn reads(&self) -> Reads {
        Reads::Everything
    }

    fn plan(&self, ctx: &PlanContext<'_>) -> Result<Plan, CommandError> {
        record(ctx, &self.id)?;
        if self.to == self.id {
            return Ok(Plan::default());
        }
        // Every built-in id rule fixes the case of its letters, and on a case-insensitive disk
        // the record's own file would be in the way: refused at once, on every OS.
        if self.to.eq_ignore_ascii_case(&self.id) {
            return Err(CommandError::CaseOnly {
                id: self.id.clone(),
                to: self.to.clone(),
            });
        }
        check_file_name(&self.to)?;
        let mut set = RenameSet::default();
        set.ids.insert(self.id.clone(), self.to.clone());
        for participant in PARTICIPANTS {
            participant.extend(ctx, &mut set)?;
        }
        check_taken(ctx, &self.to, &set)?;
        changes(ctx, set)
    }
}

/// A new id names files: it must be one name that a project path may hold.
fn check_file_name(to: &str) -> Result<(), CommandError> {
    if project_path(to)?.contains('/') {
        return Err(CommandError::Path {
            path: to.to_owned(),
            reason: "is not a file name: an id has no `/` or `\\`",
        });
    }
    Ok(())
}

/// [`CommandError::IdTaken`] for the first new id that a record has, then the first new child
/// folder, then the first new path that is on disk. The disk decides, so a hidden or ignored file
/// the index does not know also takes its path.
fn check_taken(ctx: &PlanContext<'_>, to: &str, set: &RenameSet) -> Result<(), CommandError> {
    let taken = |id: &str, path: &str| CommandError::IdTaken {
        id: id.to_owned(),
        path: path.to_owned(),
    };
    for new in set.ids.values() {
        if let Some(found) = ctx.index().record(new)?.first() {
            return Err(taken(new, &found.path));
        }
    }
    for (old, new) in &set.ids {
        if let Some(moved) = set.moves.get(&record(ctx, old)?.path) {
            let folder = join(folder_of(moved), new);
            if on_disk(ctx.root(), &folder)?.is_some() {
                return Err(taken(new, &folder));
            }
        }
    }
    for moved in set.moves.values() {
        if on_disk(ctx.root(), moved)?.is_some() {
            return Err(taken(to, moved));
        }
    }
    Ok(())
}

/// The plan of `set`: each file is read at its old path, and a source gets the edits that rename
/// the ids in it. A moved file is a removal at its old path and a creation at its new one; any
/// other file is changed in place.
fn changes(ctx: &PlanContext<'_>, set: RenameSet) -> Result<Plan, CommandError> {
    let paths: BTreeSet<&String> = set.moves.keys().chain(&set.sources).collect();
    let mut changes = Vec::new();
    for path in paths {
        let before = ctx
            .read(path)?
            .ok_or_else(|| CommandError::NotFound { what: path.clone() })?;
        let after = if set.sources.contains(path) {
            renamed(ctx, path, &before, &set.ids)?
        } else {
            before.clone()
        };
        match set.moves.get(path) {
            Some(new) => changes.extend([
                FileChange {
                    path: path.clone(),
                    before: Some(before),
                    after: None,
                },
                FileChange {
                    path: new.clone(),
                    before: None,
                    after: Some(after),
                },
            ]),
            None => changes.push(FileChange {
                path: path.clone(),
                before: Some(before),
                after: Some(after),
            }),
        }
    }
    Ok(Plan {
        changes,
        moved: set.moves,
    })
}

/// The bytes of the record `path` with the ids of `ids` renamed in its `id` field and its links,
/// as the index knows them; its own bytes when nothing in it changes.
fn renamed(
    ctx: &PlanContext<'_>,
    path: &str,
    bytes: &[u8],
    ids: &IdMap,
) -> Result<Vec<u8>, CommandError> {
    let record = parse(path, bytes)?;
    let links = ctx.index().links_from(path)?;
    let edits = edits(&record, &links, ids).map_err(|source| CommandError::Rename {
        path: path.to_owned(),
        source,
    })?;
    if edits.is_empty() {
        return Ok(bytes.to_vec());
    }
    let edited = record
        .with_edits(&edits)
        .map_err(|source| CommandError::Edit {
            path: path.to_owned(),
            source,
        })?;
    Ok(edited.text().as_bytes().to_vec())
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;
    use crate::command::failing_ops::{Fail, FailingOps, Op};
    use crate::command::files::write_plan_with;
    use crate::test_support::{Fixture, TRUTH, TRUTH_PATH, TestResult};

    fn rename(id: &str, to: &str) -> Rename {
        Rename {
            id: id.to_owned(),
            to: to.to_owned(),
        }
    }

    // AC: CE-20.3
    #[test]
    fn a_rename_plans_a_removal_a_creation_and_the_move() -> TestResult {
        let fixture = Fixture::new(&[(TRUTH_PATH, TRUTH.as_bytes())])?;
        let plan = rename("T-960", "T-970").plan(&fixture.ctx())?;
        let new = "lore/truths/T-970.md";
        assert_eq!(
            plan.changes,
            [
                FileChange {
                    path: TRUTH_PATH.to_owned(),
                    before: Some(TRUTH.as_bytes().to_vec()),
                    after: None,
                },
                FileChange {
                    path: new.to_owned(),
                    before: None,
                    after: Some(TRUTH.replace("id: T-960", "id: T-970").into_bytes()),
                },
            ]
        );
        assert_eq!(
            plan.moved,
            BTreeMap::from([(TRUTH_PATH.to_owned(), new.to_owned())])
        );
        Ok(())
    }

    // AC: CE-20.3
    #[test]
    fn a_new_child_folder_on_disk_takes_the_id() -> TestResult {
        // The folder holds no record, so only the disk knows it.
        let fixture = Fixture::new(&[
            (TRUTH_PATH, TRUTH.as_bytes()),
            ("lore/truths/T-976/notes.txt", b"synthetic notes\n"),
        ])?;
        let result = rename("T-960", "T-976").plan(&fixture.ctx());
        assert!(
            matches!(&result, Err(CommandError::IdTaken { id, path })
                if id == "T-976" && path == "lore/truths/T-976"),
            "{result:?}"
        );
        Ok(())
    }

    #[test]
    fn a_rename_whose_write_failed_plans_again() -> TestResult {
        let fixture = Fixture::new(&[
            (TRUTH_PATH, TRUTH.as_bytes()),
            ("lore/truths/T-960/notes.txt", b"synthetic notes\n"),
        ])?;
        let command = rename("T-960", "T-970");
        let plan = command.plan(&fixture.ctx())?;
        // The new folder `T-970` is written before the removal that fails.
        let mut ops = FailingOps::new(vec![Fail::new(Op::Remove, "T-960/notes.txt", 1)]);
        let result = write_plan_with(fixture.root(), &plan, &mut ops);
        assert!(
            matches!(&result, Err(CommandError::Write { path, .. })
                if path == "lore/truths/T-960/notes.txt"),
            "{result:?}"
        );
        let again = command.plan(&fixture.ctx());
        assert!(matches!(&again, Ok(again) if *again == plan), "{again:?}");
        assert!(!fixture.root().join("lore/truths/T-970").exists());
        Ok(())
    }

    #[test]
    fn a_rename_to_the_same_id_changes_nothing() -> TestResult {
        let fixture = Fixture::new(&[(TRUTH_PATH, TRUTH.as_bytes())])?;
        assert_eq!(
            rename("T-960", "T-960").plan(&fixture.ctx())?,
            Plan::default()
        );
        Ok(())
    }

    #[test]
    fn a_new_id_that_is_not_a_file_name_is_refused() -> TestResult {
        let fixture = Fixture::new(&[(TRUTH_PATH, TRUTH.as_bytes())])?;
        for to in ["sub/T-970", "sub\\T-970", "", "../T-970", "T-970."] {
            let result = rename("T-960", to).plan(&fixture.ctx());
            assert!(
                matches!(result, Err(CommandError::Path { .. })),
                "{to:?}: {result:?}"
            );
        }
        Ok(())
    }

    #[test]
    fn a_record_with_nothing_to_rename_keeps_its_bytes() -> TestResult {
        let crlf = TRUTH.replace('\n', "\r\n");
        let fixture = Fixture::new(&[(TRUTH_PATH, crlf.as_bytes())])?;
        let ids = IdMap::from([("T-999".to_owned(), "T-998".to_owned())]);
        let bytes = renamed(&fixture.ctx(), TRUTH_PATH, crlf.as_bytes(), &ids)?;
        assert_eq!(bytes, crlf.as_bytes());
        Ok(())
    }
}
