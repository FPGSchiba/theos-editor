//! The pipeline every write runs through: the fixtures, an applied command, the base check right
//! before the write, an empty plan, the repo lock and the route seam.

use std::collections::BTreeSet;
use std::time::Duration;

use theos_content::{Finding, Severity};
use theos_studio::lock::{self, COMMAND_WAIT, LockError};
use theos_studio::{
    Applied, Base, Command, CommandError, Create, Edit, EntryId, EntrySummary, FileChange, Plan,
    PlanContext, Reads, Route, Workspace, Write,
};

use crate::common::{TempProject, base_of, cold_dump, files, open, set};

type Error = Box<dyn std::error::Error>;
type TestResult = Result<(), Error>;

const TRUTH: &str = "lore/truths/T-960.md";

/// The (path, rule) of every error in `findings`.
fn errors(findings: &[Finding]) -> BTreeSet<(String, String)> {
    findings
        .iter()
        .filter(|f| f.severity == Severity::Error)
        .map(|f| (f.path.clone(), f.rule.clone()))
        .collect()
}

#[test]
fn the_fixtures_have_no_errors() -> TestResult {
    let (_project, workspace) = open("commands")?;
    let findings = workspace.index().findings()?;
    assert_eq!(errors(&findings), BTreeSet::new(), "{findings:#?}");
    Ok(())
}

#[test]
fn the_standing_fixture_has_exactly_its_standing_errors() -> TestResult {
    let (_project, workspace) = open("standing")?;
    let findings = workspace.index().findings()?;
    let expected: BTreeSet<(String, String)> = [
        ("lore/clues/C-980.md", "lore/no-overshoot"),
        ("lore/truths/T-980.md", "lore/three-clue"),
        ("lore/truths/T-981.md", "lore/three-clue"),
        ("story/scripts/S-980.md", "link/dangling"),
    ]
    .into_iter()
    .map(|(path, rule)| (path.to_owned(), rule.to_owned()))
    .collect();
    assert_eq!(errors(&findings), expected, "{findings:#?}");
    Ok(())
}

#[test]
fn an_applied_command_updates_the_index_and_returns_its_outcome() -> TestResult {
    let (project, mut workspace) = open("commands")?;
    let command = set(&workspace, TRUTH, "title", "Synthetic truth, retitled")?;
    let outcome = workspace.run(&command)?;

    assert_eq!(outcome.changed, [TRUTH]);
    assert!(outcome.reindexed);
    let text = std::fs::read_to_string(project.root().join(TRUTH))?;
    assert!(
        text.contains("title: Synthetic truth, retitled\n"),
        "{text}"
    );
    assert!(
        outcome.update.reparsed.iter().any(|path| path == TRUTH),
        "{:#?}",
        outcome.update
    );
    let records = workspace.index().record("T-960")?;
    assert_eq!(
        records.first().and_then(|r| r.title.as_deref()),
        Some("Synthetic truth, retitled")
    );
    // The truth's standing three-clue warning comes back, and nothing is new.
    assert!(
        outcome
            .findings
            .iter()
            .any(|f| f.path == TRUTH && f.rule == "lore/three-clue"),
        "{:#?}",
        outcome.findings
    );
    assert_eq!(outcome.introduced, Vec::<Finding>::new());

    let entry = outcome.entry.ok_or("no journal entry")?;
    assert_eq!(
        workspace.undo_entries(),
        [EntrySummary {
            id: entry,
            label: format!("patch {TRUTH}"),
            paths: vec![TRUTH.to_owned()],
        }]
    );
    assert_eq!(workspace.redo_entries(), Vec::<EntrySummary>::new());

    // The cache matches the disk: a cold build of a copy dumps the same.
    assert_eq!(cold_dump(project.root())?, workspace.index().dump()?);
    Ok(())
}

#[test]
fn a_new_warning_is_applied_and_introduced() -> TestResult {
    let (_project, mut workspace) = open("commands")?;
    let path = "lore/truths/T-962.md";
    let command = Create {
        path: path.to_owned(),
        text: "---\n# synthetic fixture (P06), not canon\nid: T-962\nkind: truth\n\
               title: Synthetic truth without clues\nstatus: draft\ntarget: hinted\n---\n\
               Synthetic statement.\n"
            .to_owned(),
    };
    let outcome = workspace.run(&command)?;
    assert_eq!(outcome.changed, [path]);
    let introduced: Vec<(&str, &str, Severity)> = outcome
        .introduced
        .iter()
        .map(|f| (f.path.as_str(), f.rule.as_str(), f.severity))
        .collect();
    assert_eq!(
        introduced,
        [(path, "lore/three-clue", Severity::Warning)],
        "{:#?}",
        outcome.introduced
    );
    assert!(outcome.findings.contains(&outcome.introduced[0]));
    Ok(())
}

/// A command that plans a change of `path` and then, before the pipeline writes, lets another
/// program write `other` to it.
struct RacedEdit {
    path: String,
    other: Vec<u8>,
}

impl Command for RacedEdit {
    fn label(&self) -> String {
        format!("raced edit {}", self.path)
    }

    fn reads(&self) -> Reads {
        Reads::Paths(vec![self.path.clone()])
    }

    fn plan(&self, ctx: &PlanContext<'_>) -> Result<Plan, CommandError> {
        let before = ctx.read(&self.path)?;
        let mut after = before.clone().unwrap_or_default();
        after.extend_from_slice(b"\nSynthetic text of the command.\n");
        std::fs::write(ctx.root().join(&self.path), &self.other).map_err(|source| {
            CommandError::Write {
                path: self.path.clone(),
                source,
            }
        })?;
        Ok(Plan {
            changes: vec![FileChange {
                path: self.path.clone(),
                before,
                after: Some(after),
            }],
            ..Plan::default()
        })
    }
}

#[test]
fn a_file_changed_after_the_plan_is_stale() -> TestResult {
    let (project, mut workspace) = open("commands")?;
    let mut other = std::fs::read(project.root().join(TRUTH))?;
    other.extend_from_slice(b"\nSynthetic text of another program.\n");
    let command = RacedEdit {
        path: TRUTH.to_owned(),
        other: other.clone(),
    };
    let result = workspace.run(&command);
    assert!(
        matches!(&result, Err(CommandError::Stale { path }) if path == TRUTH),
        "{result:?}"
    );
    assert_eq!(std::fs::read(project.root().join(TRUTH))?, other);
    assert_eq!(workspace.undo_entries(), Vec::<EntrySummary>::new());
    Ok(())
}

/// A truth record (LF) of its own, for the tests of a change that changes nothing.
const SYNTHETIC_TRUTH: &str = "---\n# synthetic fixture (P06), not canon\nid: T-960\nkind: truth\n\
    title: Synthetic truth\nstatus: draft\ntarget: hinted\n---\nSynthetic statement.\n";

/// A project that holds only the truth `TRUTH` with `bytes`, and a workspace on it.
fn lone_truth(bytes: &str) -> Result<(TempProject, Workspace), Error> {
    let project = TempProject::new()?;
    let full = project.root().join(TRUTH);
    std::fs::create_dir_all(full.parent().ok_or("no parent")?)?;
    std::fs::write(&full, bytes)?;
    let workspace = Workspace::open(project.root())?;
    Ok((project, workspace))
}

#[test]
fn an_unchanged_save_writes_nothing() -> TestResult {
    let lf = SYNTHETIC_TRUTH;
    let crlf = lf.replace('\n', "\r\n");
    let (project, mut workspace) = lone_truth(&crlf)?;
    let full = project.root().join(TRUTH);
    let (bytes, base) = workspace.read(TRUTH)?.ok_or("no record")?;
    assert_eq!(bytes, crlf.as_bytes());
    assert_eq!(base, Base::of(crlf.as_bytes()));

    let outcome = workspace.run(&Write {
        path: TRUTH.to_owned(),
        base,
        text: lf.to_owned(),
    })?;
    assert_eq!(outcome.changed, Vec::<String>::new());
    assert_eq!(outcome.entry, None);
    assert_eq!(std::fs::read(&full)?, crlf.as_bytes());
    assert_eq!(workspace.undo_entries(), Vec::<EntrySummary>::new());
    Ok(())
}

/// Review focus: a plan that changes nothing. A patch that sets a field to its own value and an
/// edit without edits leave an LF and a CRLF record as they are: nothing is written, and no
/// journal entry is added.
#[test]
fn a_patch_or_an_edit_that_changes_nothing_writes_nothing() -> TestResult {
    for line_end in ["\n", "\r\n"] {
        let bytes = SYNTHETIC_TRUTH.replace('\n', line_end);
        let (project, mut workspace) = lone_truth(&bytes)?;
        let same = set(&workspace, TRUTH, "title", "Synthetic truth")?;
        let none = Edit {
            path: TRUTH.to_owned(),
            base: base_of(&workspace, TRUTH)?,
            edits: Vec::new(),
        };
        for command in [&same as &dyn Command, &none] {
            let outcome = workspace.run(command)?;
            let label = command.label();
            assert_eq!(
                outcome.changed,
                Vec::<String>::new(),
                "{label} {line_end:?}"
            );
            assert_eq!(outcome.entry, None, "{label} {line_end:?}");
            assert_eq!(std::fs::read(project.root().join(TRUTH))?, bytes.as_bytes());
            assert_eq!(workspace.undo_entries(), Vec::<EntrySummary>::new());
        }
    }
    Ok(())
}

#[test]
fn a_held_lock_makes_the_command_busy() -> TestResult {
    let (project, mut workspace) = open("commands")?;
    let command = set(&workspace, TRUTH, "title", "Synthetic truth, retitled")?;
    let before = files(project.root())?;
    let _held = lock::acquire(project.root(), "synthetic holder", COMMAND_WAIT)?;
    workspace.set_lock_wait(Duration::from_millis(200));
    let result = workspace.run(&command);
    assert!(
        matches!(result, Err(CommandError::Lock(LockError::Busy { .. }))),
        "{result:?}"
    );
    assert_eq!(files(project.root())?, before);
    assert_eq!(workspace.undo_entries(), Vec::<EntrySummary>::new());
    Ok(())
}

#[test]
fn the_lock_is_released_after_every_command() -> TestResult {
    let (project, mut workspace) = open("commands")?;
    let rejected = set(&workspace, TRUTH, "target", "maybe")?;
    assert!(matches!(
        workspace.run(&rejected),
        Err(CommandError::Rejected { .. })
    ));
    drop(lock::acquire(project.root(), "probe", Duration::ZERO)?);

    let applied = set(&workspace, TRUTH, "title", "Synthetic truth, retitled")?;
    workspace.run(&applied)?;
    drop(lock::acquire(project.root(), "probe", Duration::ZERO)?);

    // The workspace no longer counts itself as the holder: the next command takes the lock.
    let again = set(
        &workspace,
        TRUTH,
        "title",
        "Synthetic truth, retitled twice",
    )?;
    workspace.run(&again)?;
    assert_eq!(workspace.undo_entries().len(), 2);
    Ok(())
}

/// A route that holds no lock and keeps no journal; it records what it was handed.
#[derive(Default)]
struct Recording {
    finished: Vec<Applied>,
}

impl Route for Recording {
    fn holds_lock(&self) -> bool {
        false
    }

    fn finish(
        &mut self,
        _workspace: &mut Workspace,
        applied: Applied,
    ) -> Result<Option<EntryId>, CommandError> {
        self.finished.push(applied);
        Ok(None)
    }
}

#[test]
fn another_route_runs_the_same_pipeline() -> TestResult {
    let (project, mut workspace) = open("commands")?;
    let before = std::fs::read(project.root().join(TRUTH))?;
    let command = set(&workspace, TRUTH, "title", "Synthetic truth, retitled")?;
    // A route that holds no lock runs while another handle holds it.
    let _held = lock::acquire(project.root(), "synthetic holder", COMMAND_WAIT)?;
    let mut route = Recording::default();
    let outcome = workspace.run_with(&command, &mut route)?;
    assert_eq!(outcome.changed, [TRUTH]);
    assert_eq!(outcome.entry, None);
    assert_eq!(workspace.undo_entries(), Vec::<EntrySummary>::new());
    let [applied] = route.finished.as_slice() else {
        return Err(format!("finished {} times", route.finished.len()).into());
    };
    assert_eq!(applied.label, format!("patch {TRUTH}"));
    assert_eq!(applied.changes.len(), 1);
    assert_eq!(applied.changes[0].path, TRUTH);
    assert_eq!(
        applied.changes[0].before.as_deref(),
        Some(before.as_slice())
    );
    assert_eq!(applied.counts_before.count(TRUTH, "lore/three-clue"), 1);
    assert_eq!(applied.counts_after.count(TRUTH, "lore/three-clue"), 1);

    // The same verdict: a route that keeps no journal still cannot write an error.
    let rejected = set(&workspace, TRUTH, "target", "maybe")?;
    assert!(matches!(
        workspace.run_with(&rejected, &mut route),
        Err(CommandError::Rejected { .. })
    ));
    assert_eq!(route.finished.len(), 1);
    Ok(())
}
