//! Undo and redo: the exact bytes come back, a later change on disk stops an undo, and an undo
//! that would bring an error is rejected.

use proptest::prelude::*;
use proptest::test_runner::TestCaseError;
use theos_content::{Span, TextEdit};
use theos_studio::{
    Command, CommandError, Create, Delete, Edit, EntrySummary, Outcome, Scope, Workspace, Write,
};

use crate::common::{TempProject, cold_dump, files, open, set};

type Error = Box<dyn std::error::Error>;
type TestResult = Result<(), Error>;

const TRUTH: &str = "lore/truths/T-960.md";
const OTHER_TRUTH: &str = "lore/truths/T-961.md";
const SCRIPT: &str = "story/scripts/S-960.md";
const NEW_TRUTH: &str = "lore/truths/T-962.md";
const STANDING_SCRIPT: &str = "story/scripts/S-980.md";
const CRLF: &str = "lore/beliefs/B-970.md";
const BARE: &str = "lore/beliefs/B-971.md";

/// A record with CRLF line ends.
const CRLF_TEXT: &str = "---\r\n# synthetic fixture (P06), not canon\r\nid: B-970\r\nkind: belief\r\n\
    title: Synthetic belief, CRLF\r\nstatus: draft\r\nheld_by: ['synthetic-singer']\r\n\
    ---\r\nSynthetic stance, CRLF.\r\n";

/// A record without a final newline.
const BARE_TEXT: &str = "---\n# synthetic fixture (P06), not canon\nid: B-971\nkind: belief\n\
    title: Synthetic belief, bare\nstatus: draft\nheld_by: ['synthetic-singer']\n\
    ---\nSynthetic stance, no final newline.";

/// Writes the two line-end records into the project; a checked-in copy could be normalised by
/// git.
fn write_line_end_records(project: &TempProject) -> TestResult {
    std::fs::write(project.root().join(CRLF), CRLF_TEXT)?;
    std::fs::write(project.root().join(BARE), BARE_TEXT)?;
    Ok(())
}

/// An edit that appends `text` to the record `path`.
fn append(workspace: &Workspace, path: &str, text: &str) -> Result<Edit, Error> {
    let (bytes, base) = workspace.read(path)?.ok_or("the file is missing")?;
    let end = String::from_utf8(bytes)?.replace("\r\n", "\n").len();
    Ok(Edit {
        path: path.to_owned(),
        base,
        edits: vec![TextEdit {
            span: Span { start: end, end },
            replacement: text.to_owned(),
        }],
    })
}

/// A write that replaces the record `path` by its text and one more line.
fn rewrite(workspace: &Workspace, path: &str) -> Result<Write, Error> {
    let (bytes, base) = workspace.read(path)?.ok_or("the file is missing")?;
    let mut text = String::from_utf8(bytes)?.replace("\r\n", "\n");
    text.push_str("Synthetic added line.\n");
    Ok(Write {
        path: path.to_owned(),
        base,
        text,
    })
}

fn delete(workspace: &Workspace, path: &str) -> Result<Delete, Error> {
    let (_, base) = workspace.read(path)?.ok_or("the file is missing")?;
    Ok(Delete {
        path: path.to_owned(),
        base,
    })
}

/// A truth without clues, as `Create` takes it.
fn new_truth(id: &str) -> Create {
    Create {
        path: format!("lore/truths/{id}.md"),
        text: format!(
            "---\n# synthetic fixture (P06), not canon\nid: {id}\nkind: truth\n\
             title: Synthetic truth without clues\nstatus: draft\ntarget: hinted\n---\n\
             Synthetic statement.\n"
        ),
    }
}

/// Runs `command`, undoes it, and checks the files and the index are as before.
fn undo_restores(
    project: &TempProject,
    workspace: &mut Workspace,
    command: &dyn Command,
) -> TestResult {
    let start = files(project.root())?;
    let applied = workspace.run(command)?;
    assert_ne!(files(project.root())?, start, "the command changed nothing");
    let undone = workspace.undo(Scope::Newest)?;
    assert_eq!(undone.entry, applied.entry);
    assert_eq!(files(project.root())?, start);
    assert_eq!(cold_dump(project.root())?, workspace.index().dump()?);
    Ok(())
}

// AC: CE-20.2
#[test]
fn undo_gives_back_the_exact_bytes() -> TestResult {
    let (project, mut workspace) = open("commands")?;

    undo_restores(&project, &mut workspace, &new_truth("T-962"))?;
    let command = rewrite(&workspace, TRUTH)?;
    undo_restores(&project, &mut workspace, &command)?;
    let command = append(&workspace, TRUTH, "\nSynthetic appended line.\n")?;
    undo_restores(&project, &mut workspace, &command)?;
    let command = set(&workspace, TRUTH, "title", "Synthetic truth, retitled")?;
    undo_restores(&project, &mut workspace, &command)?;
    let command = delete(&workspace, OTHER_TRUTH)?;
    undo_restores(&project, &mut workspace, &command)?;

    // Record files with CRLF line ends, and without a final newline. The test writes them before
    // it opens the project: git could normalise a checked-in copy.
    let (project, mut workspace) = {
        let project = TempProject::copy_of_studio("commands")?;
        write_line_end_records(&project)?;
        let workspace = Workspace::open(project.root())?;
        (project, workspace)
    };
    for path in [CRLF, BARE] {
        let command = set(&workspace, path, "title", "Synthetic belief, retitled")?;
        undo_restores(&project, &mut workspace, &command)?;
        let command = append(&workspace, path, "\nSynthetic appended line.\n")?;
        undo_restores(&project, &mut workspace, &command)?;
    }
    assert_eq!(
        std::fs::read(project.root().join(CRLF))?,
        CRLF_TEXT.as_bytes()
    );
    assert_eq!(
        std::fs::read(project.root().join(BARE))?,
        BARE_TEXT.as_bytes()
    );
    Ok(())
}

// AC: CE-20.2
#[test]
fn an_undo_after_a_later_change_fails_with_changed() -> TestResult {
    let (project, mut workspace) = open("commands")?;
    let command = set(&workspace, TRUTH, "title", "Synthetic truth, retitled")?;
    workspace.run(&command)?;
    let path = project.root().join(TRUTH);
    let mut later = std::fs::read(&path)?;
    later.extend_from_slice(b"Synthetic text from another program.\n");
    std::fs::write(&path, &later)?;

    let error = workspace.undo(Scope::Newest).err().ok_or("undo applied")?;
    assert!(
        matches!(&error, CommandError::Changed { path } if path == TRUTH),
        "{error:?}"
    );
    assert_eq!(
        error.to_string(),
        format!(
            "`{TRUTH}` changed after that command; undoing or redoing it would overwrite the \
             later change"
        )
    );
    assert_eq!(std::fs::read(&path)?, later);
    assert_eq!(workspace.undo_entries().len(), 1);
    assert_eq!(workspace.redo_entries(), Vec::<EntrySummary>::new());
    Ok(())
}

#[test]
fn a_redo_after_a_later_change_fails_with_changed() -> TestResult {
    let (project, mut workspace) = open("commands")?;
    let command = set(&workspace, TRUTH, "title", "Synthetic truth, retitled")?;
    workspace.run(&command)?;
    workspace.undo(Scope::Newest)?;
    let path = project.root().join(TRUTH);
    let mut later = std::fs::read(&path)?;
    later.extend_from_slice(b"Synthetic text from another program.\n");
    std::fs::write(&path, &later)?;

    let error = workspace.redo(Scope::Newest).err().ok_or("redo applied")?;
    assert!(
        matches!(&error, CommandError::Changed { path } if path == TRUTH),
        "{error:?}"
    );
    assert!(
        error
            .to_string()
            .ends_with("undoing or redoing it would overwrite the later change"),
        "{error}"
    );
    assert_eq!(std::fs::read(&path)?, later);
    assert_eq!(workspace.redo_entries().len(), 1);
    Ok(())
}

// AC: CE-20.2
#[test]
fn redo_gives_the_later_bytes_again() -> TestResult {
    let (project, mut workspace) = open("commands")?;
    let command = set(&workspace, TRUTH, "title", "Synthetic truth, retitled")?;
    let applied = workspace.run(&command)?;
    let later = files(project.root())?;
    let undone = workspace.undo(Scope::Newest)?;
    assert_eq!(workspace.undo_entries(), Vec::<EntrySummary>::new());
    let entry = undone.entry.ok_or("no entry")?;
    let redo: Vec<_> = workspace.redo_entries().iter().map(|e| e.id).collect();
    assert_eq!(redo, [entry]);

    let redone = workspace.redo(Scope::Newest)?;
    assert_eq!(redone.entry, applied.entry);
    assert_eq!(files(project.root())?, later);
    assert_eq!(cold_dump(project.root())?, workspace.index().dump()?);
    assert_eq!(workspace.undo_entries().len(), 1);
    assert_eq!(workspace.redo_entries(), Vec::<EntrySummary>::new());
    Ok(())
}

// AC: CE-20.2
#[test]
fn a_new_command_clears_redo() -> TestResult {
    let (_project, mut workspace) = open("commands")?;
    let command = set(&workspace, TRUTH, "title", "Synthetic truth, retitled")?;
    workspace.run(&command)?;
    workspace.undo(Scope::Newest)?;
    assert_eq!(workspace.redo_entries().len(), 1);
    let command = set(
        &workspace,
        OTHER_TRUTH,
        "title",
        "Synthetic other, retitled",
    )?;
    workspace.run(&command)?;
    assert_eq!(workspace.redo_entries(), Vec::<EntrySummary>::new());
    let error = workspace.redo(Scope::Newest).err().ok_or("redo applied")?;
    assert!(matches!(error, CommandError::NothingToRedo), "{error:?}");
    Ok(())
}

// AC: CE-20.2
#[test]
fn scope_record_undoes_the_newest_entry_of_that_record() -> TestResult {
    let (project, mut workspace) = open("commands")?;
    let start = std::fs::read(project.root().join(TRUTH))?;
    let other = std::fs::read(project.root().join(OTHER_TRUTH))?;
    let command = set(&workspace, TRUTH, "title", "Synthetic truth, retitled")?;
    workspace.run(&command)?;
    let command = set(
        &workspace,
        OTHER_TRUTH,
        "title",
        "Synthetic other, retitled",
    )?;
    let second = workspace.run(&command)?;

    workspace.undo(Scope::Record(TRUTH.to_owned()))?;
    assert_eq!(std::fs::read(project.root().join(TRUTH))?, start);
    assert_ne!(std::fs::read(project.root().join(OTHER_TRUTH))?, other);
    let remaining: Vec<_> = workspace.undo_entries().iter().map(|e| e.id).collect();
    assert_eq!(remaining, [second.entry.ok_or("no entry")?]);

    // Redo takes the same scope.
    let error = workspace
        .redo(Scope::Record(OTHER_TRUTH.to_owned()))
        .err()
        .ok_or("redo applied")?;
    assert!(matches!(error, CommandError::NothingToRedo), "{error:?}");
    workspace.redo(Scope::Record(TRUTH.to_owned()))?;
    assert_ne!(std::fs::read(project.root().join(TRUTH))?, start);
    Ok(())
}

/// Review focus, paths: `Scope::Record` reads a `\` as `/`, as every command path does, and
/// refuses a path outside the rules.
#[test]
fn scope_record_reads_a_backslash_as_a_slash() -> TestResult {
    let (project, mut workspace) = open("commands")?;
    let path = project.root().join(TRUTH);
    let start = std::fs::read(&path)?;
    let command = set(&workspace, TRUTH, "title", "Synthetic truth, retitled")?;
    workspace.run(&command)?;
    let later = std::fs::read(&path)?;
    let windows = TRUTH.replace('/', "\\");

    workspace.undo(Scope::Record(windows.clone()))?;
    assert_eq!(std::fs::read(&path)?, start);
    workspace.redo(Scope::Record(windows))?;
    assert_eq!(std::fs::read(&path)?, later);

    for bad in ["../outside.md", "lore\\..\\..\\outside.md"] {
        let error = workspace
            .undo(Scope::Record(bad.to_owned()))
            .err()
            .ok_or("undo applied")?;
        assert!(matches!(error, CommandError::Path { .. }), "{error:?}");
        let error = workspace
            .redo(Scope::Record(bad.to_owned()))
            .err()
            .ok_or("redo applied")?;
        assert!(matches!(error, CommandError::Path { .. }), "{error:?}");
    }
    assert_eq!(std::fs::read(&path)?, later);
    Ok(())
}

// AC: CE-20.2
#[test]
fn undoing_a_fix_brings_the_error_back() -> TestResult {
    let (project, mut workspace) = open("standing")?;
    let (bytes, base) = workspace.read(STANDING_SCRIPT)?.ok_or("no script")?;
    let text = String::from_utf8(bytes)?;
    let link = "[[synthetic-gone]]";
    let start = text.find(link).ok_or("no link")?;
    let command = Edit {
        path: STANDING_SCRIPT.to_owned(),
        base,
        edits: vec![TextEdit {
            span: Span {
                start,
                end: start + link.len(),
            },
            replacement: "nothing".to_owned(),
        }],
    };
    workspace.run(&command)?;
    let fixed = std::fs::read_to_string(project.root().join(STANDING_SCRIPT))?;
    assert!(!fixed.contains("synthetic-gone"), "{fixed}");

    let outcome = workspace.undo(Scope::Newest)?;
    let restored = std::fs::read_to_string(project.root().join(STANDING_SCRIPT))?;
    assert_eq!(restored, text);
    assert!(
        outcome
            .findings
            .iter()
            .any(|f| f.path == STANDING_SCRIPT && f.rule == "link/dangling"),
        "{:#?}",
        outcome.findings
    );
    Ok(())
}

// AC: CE-20.2
#[test]
fn undoing_a_linked_create_is_rejected() -> TestResult {
    let (project, mut workspace) = open("commands")?;
    workspace.run(&new_truth("T-962"))?;
    let (bytes, base) = workspace.read(SCRIPT)?.ok_or("no script")?;
    let end = bytes.len();
    let link = Edit {
        path: SCRIPT.to_owned(),
        base,
        edits: vec![TextEdit {
            span: Span { start: end, end },
            replacement: "\nSynthetic text that names [[T-962]].\n".to_owned(),
        }],
    };
    workspace.run(&link)?;
    let before = files(project.root())?;

    let error = workspace
        .undo(Scope::Record(NEW_TRUTH.to_owned()))
        .err()
        .ok_or("undo applied")?;
    match &error {
        CommandError::Rejected { blocking, .. } => assert!(
            blocking.iter().any(|f| f.rule == "link/dangling"),
            "{blocking:#?}"
        ),
        other => return Err(format!("expected Rejected, got {other:?}").into()),
    }
    assert_eq!(files(project.root())?, before);
    assert_eq!(workspace.undo_entries().len(), 2);
    Ok(())
}

#[test]
fn nothing_to_undo_and_redo() -> TestResult {
    let (_project, mut workspace) = open("commands")?;
    let error = workspace.undo(Scope::Newest).err().ok_or("undo applied")?;
    assert!(matches!(error, CommandError::NothingToUndo), "{error:?}");
    let error = workspace.redo(Scope::Newest).err().ok_or("redo applied")?;
    assert!(matches!(error, CommandError::NothingToRedo), "{error:?}");
    let command = set(&workspace, TRUTH, "title", "Synthetic truth, retitled")?;
    workspace.run(&command)?;
    let error = workspace
        .undo(Scope::Record("lore/truths/T-nothing.md".to_owned()))
        .err()
        .ok_or("undo applied")?;
    assert!(matches!(error, CommandError::NothingToUndo), "{error:?}");
    Ok(())
}

#[test]
fn the_journal_keeps_a_hundred_entries() -> TestResult {
    let project = TempProject::new()?;
    let mut workspace = Workspace::open(project.root())?;
    let mut first: Option<Outcome> = None;
    for n in 0..101 {
        let outcome = workspace.run(&new_truth(&format!("T-{}", 100 + n)))?;
        first.get_or_insert(outcome);
    }
    let entries = workspace.undo_entries();
    assert_eq!(entries.len(), 100);
    let oldest = first.and_then(|o| o.entry).ok_or("no entry")?;
    assert!(entries.iter().all(|entry| entry.id != oldest));
    assert_eq!(
        entries.last().map(|e| e.paths.clone()),
        Some(vec!["lore/truths/T-101.md".to_owned()])
    );
    Ok(())
}

/// Applies one command of kind `kind` on the record `pick` of the pool. `false` when it was
/// skipped: the record is gone, or the command was rejected.
fn step(workspace: &mut Workspace, n: usize, kind: u8, pick: u8) -> Result<bool, Error> {
    const POOL: [&str; 5] = [
        TRUTH,
        OTHER_TRUTH,
        "lore/beliefs/B-961.md",
        "lore/clues/C-961.md",
        "concepts/synthetic-shrine.md",
    ];
    let path = POOL[usize::from(pick) % POOL.len()];
    if !kind.is_multiple_of(5) && workspace.read(path)?.is_none() {
        return Ok(false);
    }
    let result = match kind % 5 {
        0 => workspace.run(&new_truth(&format!("T-{}", 200 + n))),
        1 => workspace.run(&rewrite(workspace, path)?),
        2 => workspace.run(&append(workspace, path, "\nSynthetic appended line.\n")?),
        3 => workspace.run(&set(workspace, path, "title", &format!("Synthetic {n}"))?),
        _ => workspace.run(&delete(workspace, path)?),
    };
    match result {
        Ok(_) => Ok(true),
        Err(CommandError::Rejected { .. }) => Ok(false),
        Err(other) => Err(other.into()),
    }
}

fn history_undoes_to_the_start(steps: &[(u8, u8)]) -> TestResult {
    let (project, mut workspace) = open("commands")?;
    let start = files(project.root())?;
    for (n, (kind, pick)) in steps.iter().enumerate() {
        step(&mut workspace, n, *kind, *pick)?;
    }
    loop {
        match workspace.undo(Scope::Newest) {
            Ok(_) => {}
            Err(CommandError::NothingToUndo) => break,
            Err(other) => return Err(other.into()),
        }
    }
    assert_eq!(files(project.root())?, start);
    assert_eq!(cold_dump(project.root())?, workspace.index().dump()?);
    Ok(())
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(16))]

    // Backs CE-20.2 with random histories (proptest tests are not traced).
    #[test]
    fn undoing_a_random_history_gives_the_first_bytes(
        steps in proptest::collection::vec((0u8..5, 0u8..5), 0..=8)
    ) {
        history_undoes_to_the_start(&steps)
            .map_err(|error| TestCaseError::fail(error.to_string()))?;
    }
}
