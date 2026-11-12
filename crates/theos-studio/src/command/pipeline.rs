//! The pipeline every write runs through (spec §5.4): lock, freshen and plan, base check,
//! preview, verdict, write, reindex, finish. The steps are the same for every route.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use theos_content::{Change, Finding, Index, IndexError, Preview, Update};

use super::files::write_plan;
use super::journal::EntryId;
use super::route::{Applied, Route};
use super::state::CommandState;
use super::verdict;
use super::{Command, CommandError, FileChange, Plan, PlanContext, Reads, project_path, read_file};
use crate::introduced::FindingCounts;
use crate::lock::{self, LockError, LockGuard};
use crate::{StudioError, Workspace};

/// What an applied command did.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Outcome {
    /// The paths written, in plan order.
    pub changed: Vec<String>,
    /// What the reindex did. When the reindex failed (`reindexed` is `false`), the preview's
    /// update: the change as previewed.
    pub update: Update,
    /// Every finding on the affected paths after the command, warnings and standing errors
    /// included.
    pub findings: Vec<Finding>,
    /// The findings of `findings` that are new: warnings only, since a new error rejects the
    /// command.
    pub introduced: Vec<Finding>,
    /// The journal entry, when the route keeps one.
    pub entry: Option<EntryId>,
    /// Whether the reindex after the write brought the index cache in line with the files; `true`
    /// also when nothing was written. `false` when it failed, for example because another process
    /// held the cache: the files are written and the route has finished (the journal entry is
    /// there), and the next sync or update of these paths brings the cache in line.
    pub reindexed: bool,
}

impl Outcome {
    /// The outcome of a plan that changes nothing.
    fn unchanged() -> Outcome {
        Outcome {
            changed: Vec::new(),
            update: Update::default(),
            findings: Vec::new(),
            introduced: Vec::new(),
            entry: None,
            reindexed: true,
        }
    }
}

/// What the pipeline runs.
pub(crate) enum Work<'a> {
    /// A command, planned on the workspace as it is now.
    Command(&'a dyn Command),
    /// A journal entry played back by undo or redo.
    Replay {
        /// The label of the undo or redo.
        label: String,
        /// The changes to write, from the entry.
        plan: Plan,
        /// The counts of the state the replay restores; its verdict allows them too.
        allowance: FindingCounts,
        /// The entry played back. Its outcome names it, and no new entry is added.
        entry: EntryId,
    },
}

impl Work<'_> {
    fn label(&self) -> String {
        match self {
            Work::Command(command) => command.label(),
            Work::Replay { label, .. } => label.clone(),
        }
    }
}

/// What a replay brings beyond its plan.
struct Replayed {
    allowance: FindingCounts,
    entry: EntryId,
}

/// Runs `work` through every step on `route`. The repo lock, when the route holds it, is released
/// on every way out. Every log line of the run is in the span `command`, which names the work by
/// its label.
pub(crate) fn run(
    workspace: &mut Workspace,
    work: Work<'_>,
    route: &mut dyn Route,
) -> Result<Outcome, CommandError> {
    let label = work.label();
    let _span = tracing::info_span!("command", %label).entered();
    let held = Held::take(workspace, &label, route.holds_lock())?;
    steps(held.workspace, work, label, route)
}

/// The workspace of one command, with the repo lock when its route holds it. Dropping it, on
/// every way out, a panic included, clears the workspace's flag and releases the lock.
struct Held<'w> {
    workspace: &'w mut Workspace,
    guard: Option<LockGuard>,
}

impl<'w> Held<'w> {
    /// Step 1: takes the repo lock when `lock` is set.
    fn take(workspace: &'w mut Workspace, label: &str, lock: bool) -> Result<Held<'w>, LockError> {
        let guard = if lock {
            let root = workspace.project().root().to_path_buf();
            Some(take_lock(&mut workspace.commands, &root, label)?)
        } else {
            None
        };
        Ok(Held { workspace, guard })
    }
}

impl Drop for Held<'_> {
    fn drop(&mut self) {
        if let Some(guard) = self.guard.take() {
            self.workspace.commands.locked = false;
            drop(guard);
        }
    }
}

/// The repo lock of `root`, unless this workspace holds it already: then [`LockError::Reentrant`]
/// at once, instead of waiting for itself.
fn take_lock(state: &mut CommandState, root: &Path, label: &str) -> Result<LockGuard, LockError> {
    if state.locked {
        return Err(LockError::Reentrant);
    }
    let guard = lock::acquire(root, label, state.lock_wait)?;
    state.locked = true;
    Ok(guard)
}

/// Steps 2 to 8.
fn steps(
    workspace: &mut Workspace,
    work: Work<'_>,
    label: String,
    route: &mut dyn Route,
) -> Result<Outcome, CommandError> {
    let (plan, replayed) = plan(workspace, work)?;
    if plan.is_empty() {
        tracing::debug!(%label, "nothing to change");
        return Ok(Outcome::unchanged());
    }
    let root = workspace.project().root().to_path_buf();
    check_bases(&root, &plan, replayed.is_some())?;
    let (preview, before) = preview(workspace, &plan)?;
    let allowance = replayed.as_ref().map(|replayed| &replayed.allowance);
    let verdict = verdict::judge(&before, &plan.moved, allowance, &preview.findings);
    if !verdict.blocking.is_empty() {
        let blocking: Vec<String> = verdict.blocking.iter().map(ToString::to_string).collect();
        tracing::info!(%label, paths = ?plan.paths(), ?blocking, "command rejected");
        return Err(CommandError::Rejected {
            blocking: verdict.blocking,
            findings: preview.findings,
        });
    }
    let written = write_plan(&root, &plan);
    // From here the files are written: the route finishes whatever the reindex does.
    let (update, reindexed) = match reindex(workspace, &label, &plan, written)? {
        Some(update) => (update, true),
        None => (preview.update, false),
    };
    let changes: Vec<FileChange> = plan
        .changes
        .into_iter()
        .filter(FileChange::is_real)
        .collect();
    let paths: Vec<String> = changes.iter().map(|change| change.path.clone()).collect();
    tracing::info!(%label, ?paths, "command applied");
    // Step 8. A replay moves its own entry between the lists; it adds none.
    let entry = if let Some(replayed) = replayed {
        Some(replayed.entry)
    } else {
        let applied = Applied {
            label,
            changes,
            moved: plan.moved,
            counts_before: FindingCounts::of(&before, &BTreeMap::new()),
            counts_after: FindingCounts::of(&preview.findings, &BTreeMap::new()),
        };
        route.finish(workspace, applied)?
    };
    Ok(Outcome {
        changed: paths,
        update,
        findings: preview.findings,
        introduced: verdict.introduced,
        entry,
        reindexed,
    })
}

/// Step 2: brings the index in line with the disk on what the work reads, then plans. A replay's
/// plan comes from its entry; the index is freshened on its paths.
fn plan(
    workspace: &mut Workspace,
    work: Work<'_>,
) -> Result<(Plan, Option<Replayed>), CommandError> {
    match work {
        Work::Command(command) => {
            freshen(workspace, &command.reads())?;
            let ctx = PlanContext::new(
                workspace.project().root(),
                workspace.index(),
                workspace.registry(),
            );
            Ok((command.plan(&ctx)?, None))
        }
        Work::Replay {
            plan,
            allowance,
            entry,
            ..
        } => {
            freshen(workspace, &Reads::Paths(plan.paths()))?;
            Ok((plan, Some(Replayed { allowance, entry })))
        }
    }
}

/// Updates the index on the paths `reads` names, or syncs it whole. A path that breaks the
/// rules fails with [`CommandError::Path`] before anything is read.
fn freshen(workspace: &mut Workspace, reads: &Reads) -> Result<(), CommandError> {
    let update = match reads {
        Reads::Paths(paths) => {
            let paths = paths
                .iter()
                .map(|path| project_path(path))
                .collect::<Result<Vec<String>, CommandError>>()?;
            let paths: Vec<&str> = paths.iter().map(String::as_str).collect();
            on_index(workspace, |index| index.update(&paths))?
        }
        Reads::Everything => on_index(workspace, Index::sync)?,
    };
    tracing::debug!(?update, "freshened the index");
    Ok(())
}

/// Step 3: every file of the plan must hold its `before` right now; another program may have
/// written it since the plan. A difference is [`CommandError::Stale`], for a replay
/// [`CommandError::Changed`].
fn check_bases(root: &Path, plan: &Plan, replay: bool) -> Result<(), CommandError> {
    for change in &plan.changes {
        if read_file(root, &change.path)? != change.before {
            let path = change.path.clone();
            return Err(if replay {
                CommandError::Changed { path }
            } else {
                CommandError::Stale { path }
            });
        }
    }
    Ok(())
}

/// Step 4: the preview of the plan, then the findings the affected paths have now, read on the
/// unchanged cache because the preview rolls back. A file the plan moves is looked up under its
/// old path.
fn preview(
    workspace: &mut Workspace,
    plan: &Plan,
) -> Result<(Preview, Vec<Finding>), CommandError> {
    let changes: Vec<Change> = plan.changes.iter().map(to_change).collect();
    let preview = on_index(workspace, |index| index.preview(&changes))?;
    let old_of: BTreeMap<&str, &str> = plan
        .moved
        .iter()
        .map(|(old, new)| (new.as_str(), old.as_str()))
        .collect();
    let lookups: BTreeSet<&str> = affected(&preview, plan)
        .into_iter()
        .map(|path| old_of.get(path).copied().unwrap_or(path))
        .collect();
    let before = on_index(workspace, |index| {
        let mut found = Vec::new();
        for path in &lookups {
            found.extend(index.findings_for(path)?);
        }
        Ok(found)
    })?;
    Ok((preview, before))
}

/// The paths a command affects (spec §5.5): what the preview re-parsed, relinked, revalidated or
/// removed, every path the plan gives content for, and every path the preview has a finding on.
fn affected<'a>(preview: &'a Preview, plan: &'a Plan) -> BTreeSet<&'a str> {
    let update = &preview.update;
    let listed = update
        .reparsed
        .iter()
        .chain(&update.relinked)
        .chain(&update.revalidated)
        .chain(&update.removed);
    let content = plan
        .changes
        .iter()
        .filter(|change| change.after.is_some())
        .map(|change| &change.path);
    let found = preview.findings.iter().map(|finding| &finding.path);
    listed
        .chain(content)
        .chain(found)
        .map(String::as_str)
        .collect()
}

/// A change of the plan as the index takes it.
fn to_change(change: &FileChange) -> Change {
    match &change.after {
        Some(bytes) => Change::Content {
            path: change.path.clone(),
            bytes: bytes.clone(),
        },
        None => Change::Removed {
            path: change.path.clone(),
        },
    }
}

/// Step 7: brings the index in line with the disk on every path of the plan, also after a
/// failed write, so the cache matches the disk. A failed write's error comes first. After a
/// successful write a reindex failure is no error: it is logged, and `None` says the cache is
/// behind the files until the next sync or update.
fn reindex(
    workspace: &mut Workspace,
    label: &str,
    plan: &Plan,
    written: Result<(), CommandError>,
) -> Result<Option<Update>, CommandError> {
    let paths = plan.paths();
    let paths: Vec<&str> = paths.iter().map(String::as_str).collect();
    let update = update_paths(workspace, &paths);
    let update = match (written, update) {
        (Ok(()), Ok(update)) => update,
        (Ok(()), Err(error)) => {
            tracing::warn!(%label, %error, "cannot reindex after the write; the next sync catches up");
            return Ok(None);
        }
        (Err(error), Ok(_)) => return Err(error),
        (Err(error), Err(reindex)) => {
            tracing::warn!(%label, %reindex, "cannot reindex after the failed write");
            return Err(error);
        }
    };
    tracing::debug!(
        read = ?update.read,
        reparsed = ?update.reparsed,
        removed = ?update.removed,
        relinked = ?update.relinked,
        revalidated = ?update.revalidated,
        "reindexed"
    );
    Ok(Some(update))
}

/// `index.update` on `paths`. Under test, [`CommandState`]'s `fail_reindex` makes it fail as a
/// busy cache does.
fn update_paths(workspace: &mut Workspace, paths: &[&str]) -> Result<Update, CommandError> {
    #[cfg(test)]
    if workspace.commands.fail_reindex {
        return Err(CommandError::Index(IndexError::Busy {
            path: std::path::PathBuf::from("index.sqlite"),
        }));
    }
    on_index(workspace, |index| index.update(paths))
}

/// Runs `f` on the index through [`Workspace::write`]; an index failure stays
/// [`CommandError::Index`].
fn on_index<T>(
    workspace: &mut Workspace,
    f: impl FnMut(&mut Index) -> Result<T, IndexError>,
) -> Result<T, CommandError> {
    workspace.write(f).map_err(|error| match error {
        StudioError::Index(error) => CommandError::Index(error),
        other => CommandError::Studio(other),
    })
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;
    use crate::command::HumanRoute;
    use crate::command::test_support::{Fixture, TRUTH, TestResult};

    #[test]
    fn reentry_is_refused() -> TestResult {
        let dir = tempfile::tempdir()?;
        let mut state = CommandState {
            locked: true,
            ..CommandState::default()
        };
        let result = take_lock(&mut state, dir.path(), "again");
        assert!(matches!(result, Err(LockError::Reentrant)), "{result:?}");
        assert!(state.locked);
        // The refused call did not take the lock.
        drop(lock::acquire(dir.path(), "probe", Duration::ZERO)?);
        Ok(())
    }

    #[test]
    fn the_lock_step_sets_the_flag_and_the_guard_clears_it() -> TestResult {
        let mut fixture = Fixture::new(&[])?;
        let root = fixture.root().to_path_buf();
        {
            let held = Held::take(fixture.workspace_mut(), "synthetic", true)?;
            assert!(held.workspace.commands.locked);
            assert!(matches!(
                lock::acquire(&root, "probe", Duration::ZERO),
                Err(LockError::Busy { .. })
            ));
        }
        assert!(!fixture.workspace_mut().commands.locked);
        drop(lock::acquire(&root, "probe", Duration::ZERO)?);
        Ok(())
    }

    fn replay(before: &[u8], after: &[u8]) -> Work<'static> {
        Work::Replay {
            label: "undo: synthetic".to_owned(),
            plan: Plan {
                changes: vec![FileChange {
                    path: "notes.txt".to_owned(),
                    before: Some(before.to_vec()),
                    after: Some(after.to_vec()),
                }],
                ..Plan::default()
            },
            allowance: FindingCounts::default(),
            entry: EntryId(7),
        }
    }

    #[test]
    fn a_replay_whose_file_changed_fails_with_changed() -> TestResult {
        let mut fixture = Fixture::new(&[("notes.txt", b"later\n")])?;
        let result = run(
            fixture.workspace_mut(),
            replay(b"one\n", b"two\n"),
            &mut HumanRoute,
        );
        assert!(
            matches!(&result, Err(CommandError::Changed { path }) if path == "notes.txt"),
            "{result:?}"
        );
        assert_eq!(std::fs::read(fixture.root().join("notes.txt"))?, b"later\n");
        Ok(())
    }

    #[test]
    fn a_failed_reindex_after_the_write_keeps_the_entry() -> TestResult {
        let path = "lore/truths/T-960.md";
        let mut fixture = Fixture::new(&[(path, TRUTH.as_bytes())])?;
        let root = fixture.root().to_path_buf();
        let after = TRUTH.replace("title: Old", "title: New");
        let command = crate::Write {
            path: path.to_owned(),
            base: crate::Base::of(TRUTH.as_bytes()),
            text: after.clone(),
        };
        let workspace = fixture.workspace_mut();
        workspace.commands.fail_reindex = true;
        let outcome = workspace.run(&command)?;

        assert!(!outcome.reindexed);
        assert_eq!(outcome.changed, [path]);
        // The update is the preview's: the change as previewed.
        assert_eq!(outcome.update.reparsed, [path]);
        let entry = outcome.entry.ok_or("no journal entry")?;
        let ids: Vec<EntryId> = workspace.undo_entries().iter().map(|e| e.id).collect();
        assert_eq!(ids, [entry]);
        assert_eq!(std::fs::read(root.join(path))?, after.as_bytes());

        // The cache keeps the old record until the next update brings it in line.
        let title = |workspace: &Workspace| -> Result<Option<String>, IndexError> {
            Ok(workspace
                .index()
                .record("T-960")?
                .pop()
                .and_then(|r| r.title))
        };
        assert_eq!(title(workspace)?.as_deref(), Some("Old"));
        workspace.commands.fail_reindex = false;
        workspace.write(|index| index.update(&[path]))?;
        assert_eq!(title(workspace)?.as_deref(), Some("New"));
        assert!(!workspace.commands.locked);
        Ok(())
    }

    #[test]
    fn a_replay_names_its_entry_and_adds_none() -> TestResult {
        let mut fixture = Fixture::new(&[("notes.txt", b"one\n")])?;
        let outcome = run(
            fixture.workspace_mut(),
            replay(b"one\n", b"two\n"),
            &mut HumanRoute,
        )?;
        assert_eq!(outcome.entry, Some(EntryId(7)));
        assert_eq!(outcome.changed, ["notes.txt"]);
        assert_eq!(
            fixture.workspace_mut().undo_entries(),
            Vec::<crate::EntrySummary>::new()
        );
        assert_eq!(std::fs::read(fixture.root().join("notes.txt"))?, b"two\n");
        Ok(())
    }
}
