//! CE-20.3, rename: one command renames an id, its file, its child folder, a concept's versions
//! and every link the index knows to them. CE-20.2: undoing it puts every path back.

use std::collections::BTreeMap;

use theos_content::Finding;
use theos_studio::{CommandError, EntrySummary, Outcome, Rename, Scope, Workspace};

use crate::common::{TempProject, cold_dump, files, open};

type Error = Box<dyn std::error::Error>;
type TestResult = Result<(), Error>;
type Files = BTreeMap<String, Vec<u8>>;

const TRUTH: &str = "lore/truths/T-960.md";
const CLUE: &str = "lore/clues/C-960.md";
const SECOND_CLUE: &str = "lore/clues/C-961.md";
const BELIEF: &str = "lore/beliefs/B-960.md";
const NPC: &str = "people/synthetic-folk/synthetic-singer.md";
const SCRIPT: &str = "story/scripts/S-960.md";
const CONCEPT: &str = "concepts/synthetic-shrine.md";
const FLOOR: &str = "world/synthetic-band/f04-synthetic.md";
const FLOOR_FOLDER: &str = "world/synthetic-band/f04-synthetic";
const RENAMED_FLOOR_FOLDER: &str = "world/synthetic-band/f04-renamed";

fn rename(id: &str, to: &str) -> Rename {
    Rename {
        id: id.to_owned(),
        to: to.to_owned(),
    }
}

/// A rename applied to a fresh copy of `commands/`, with the files from before it.
struct Renamed {
    project: TempProject,
    workspace: Workspace,
    before: Files,
}

/// Renames `id` to `to` in a fresh copy of `commands/` and checks what every rename leaves:
/// no backlink of an old id of `old_ids`, no `link/dangling`, no new finding, exactly one
/// journal entry, and an index equal to a cold build.
fn renamed(id: &str, to: &str, old_ids: &[&str]) -> Result<Renamed, Error> {
    let (project, mut workspace) = open("commands")?;
    let before = files(project.root())?;
    let outcome = workspace.run(&rename(id, to))?;
    assert_clean(&project, &workspace, &outcome, old_ids)?;
    Ok(Renamed {
        project,
        workspace,
        before,
    })
}

fn assert_clean(
    project: &TempProject,
    workspace: &Workspace,
    outcome: &Outcome,
    old_ids: &[&str],
) -> TestResult {
    for old in old_ids {
        assert_eq!(workspace.index().backlinks(old)?, Vec::new(), "{old}");
        assert_eq!(workspace.index().record(old)?, Vec::new(), "{old}");
    }
    let findings = workspace.index().findings()?;
    assert!(
        !findings.iter().any(|f| f.rule == "link/dangling"),
        "{findings:#?}"
    );
    assert_eq!(outcome.introduced, Vec::<Finding>::new());
    let entry = outcome.entry.ok_or("no journal entry")?;
    let entries: Vec<_> = workspace.undo_entries().iter().map(|e| e.id).collect();
    assert_eq!(entries, [entry]);
    assert_eq!(cold_dump(project.root())?, workspace.index().dump()?);
    Ok(())
}

/// The files a rename should leave: `before`, with each file of `moves` at its new path, then
/// the replacements of `edits` (new path → (old text, new text)) applied. A move whose old path
/// ends in `/` moves everything under that folder. Every old text must be in its file.
fn expected(
    before: &Files,
    moves: &[(&str, &str)],
    edits: &[(&str, &[(&str, &str)])],
) -> Result<Files, Error> {
    let mut expected = Files::new();
    for (path, bytes) in before {
        let exact = moves.iter().find(|(old, _)| old == path);
        let folder = moves.iter().find_map(|(old, new)| {
            let rest = path.strip_prefix(old)?;
            old.ends_with('/').then(|| format!("{new}{rest}"))
        });
        let target = match (exact, folder) {
            (Some((_, new)), _) => (*new).to_owned(),
            (None, Some(new)) => new,
            (None, None) => path.clone(),
        };
        expected.insert(target, bytes.clone());
    }
    for (path, replacements) in edits {
        let bytes = expected
            .get_mut(*path)
            .ok_or_else(|| format!("no file {path} is expected"))?;
        let mut text = String::from_utf8(bytes.clone())?;
        for (old, new) in *replacements {
            assert!(text.contains(old), "{path} has no {old:?}:\n{text}");
            text = text.replace(old, new);
        }
        *bytes = text.into_bytes();
    }
    Ok(expected)
}

/// Asserts that the project holds exactly the files of `expected`, byte for byte.
fn assert_files(project: &TempProject, expected: &Files) -> TestResult {
    let actual = files(project.root())?;
    let paths = |files: &Files| files.keys().cloned().collect::<Vec<_>>();
    assert_eq!(paths(&actual), paths(expected));
    for (path, bytes) in expected {
        let found = actual.get(path).ok_or("a file is missing")?;
        assert_eq!(
            String::from_utf8_lossy(found),
            String::from_utf8_lossy(bytes),
            "{path}"
        );
        assert_eq!(found, bytes, "{path}");
    }
    Ok(())
}

/// Runs `rename`, which must fail, and checks that it changed no file and added no entry.
fn refused(
    project: &TempProject,
    workspace: &mut Workspace,
    rename: &Rename,
) -> Result<CommandError, Error> {
    let before = files(project.root())?;
    let error = workspace.run(rename).err().ok_or("the rename applied")?;
    assert_eq!(files(project.root())?, before);
    assert_eq!(workspace.undo_entries(), Vec::<EntrySummary>::new());
    Ok(error)
}

// AC: CE-20.3
#[test]
fn renaming_a_truth_rewrites_every_link() -> TestResult {
    let new = "lore/truths/T-970.md";
    let run = renamed("T-960", "T-970", &["T-960"])?;
    let expected = expected(
        &run.before,
        &[(TRUTH, new)],
        &[
            (new, &[("\nid: T-960\n", "\nid: T-970\n")]),
            (CLUE, &[("\nreveals: T-960\n", "\nreveals: T-970\n")]),
            (
                SECOND_CLUE,
                &[("\nreveals: \"T-960\"\n", "\nreveals: \"T-970\"\n")],
            ),
            (BELIEF, &[("\ndistorts: T-960\n", "\ndistorts: T-970\n")]),
            (NPC, &[("unaware_of: [T-960]", "unaware_of: [T-970]")]),
            (
                SCRIPT,
                &[
                    ("[[T-960|the truth]]", "[[T-970|the truth]]"),
                    ("[[T-960]] and [[T-960]]", "[[T-970]] and [[T-970]]"),
                ],
            ),
        ],
    )?;
    assert_files(&run.project, &expected)?;
    Ok(())
}

// AC: CE-20.3
#[test]
fn renaming_a_script_keeps_beat_fragments() -> TestResult {
    let new = "story/scripts/S-970.md";
    let run = renamed("S-960", "S-970", &["S-960"])?;
    let expected = expected(
        &run.before,
        &[(SCRIPT, new)],
        &[
            (new, &[("\nid: S-960\n", "\nid: S-970\n")]),
            (
                TRUTH,
                &[("[[S-960#1|the arrival]]", "[[S-970#1|the arrival]]")],
            ),
        ],
    )?;
    assert_files(&run.project, &expected)?;
    Ok(())
}

/// The moves of the concept rename: the concept, and each version into the new folder.
const CONCEPT_MOVES: [(&str, &str); 4] = [
    (CONCEPT, "concepts/synthetic-temple.md"),
    (
        "concepts/synthetic-shrine/synthetic-shrine-v1.md",
        "concepts/synthetic-temple/synthetic-temple-v1.md",
    ),
    (
        "concepts/synthetic-shrine/synthetic-shrine-v2.md",
        "concepts/synthetic-temple/synthetic-temple-v2.md",
    ),
    (
        "concepts/synthetic-shrine/synthetic-shrine-v3.md",
        "concepts/synthetic-temple/synthetic-temple-v3.md",
    ),
];

const CONCEPT_IDS: [&str; 4] = [
    "synthetic-shrine",
    "synthetic-shrine-v1",
    "synthetic-shrine-v2",
    "synthetic-shrine-v3",
];

// AC: CE-20.3
#[test]
fn renaming_a_concept_renames_and_moves_its_versions() -> TestResult {
    let run = renamed("synthetic-shrine", "synthetic-temple", &CONCEPT_IDS)?;
    let concept = (
        "\nconcept: synthetic-shrine\n",
        "\nconcept: synthetic-temple\n",
    );
    let expected = expected(
        &run.before,
        &CONCEPT_MOVES,
        &[
            (
                "concepts/synthetic-temple.md",
                &[
                    ("\nid: synthetic-shrine\n", "\nid: synthetic-temple\n"),
                    (
                        "picked: [synthetic-shrine-v3]",
                        "picked: [synthetic-temple-v3]",
                    ),
                    (
                        "[[synthetic-shrine-v2]] · resolved by [[synthetic-shrine-v3]]",
                        "[[synthetic-temple-v2]] · resolved by [[synthetic-temple-v3]]",
                    ),
                ],
            ),
            (
                "concepts/synthetic-temple/synthetic-temple-v1.md",
                &[
                    ("\nid: synthetic-shrine-v1\n", "\nid: synthetic-temple-v1\n"),
                    concept,
                ],
            ),
            (
                "concepts/synthetic-temple/synthetic-temple-v2.md",
                &[
                    ("\nid: synthetic-shrine-v2\n", "\nid: synthetic-temple-v2\n"),
                    concept,
                    (
                        "\nfrom: synthetic-shrine-v1\n",
                        "\nfrom: synthetic-temple-v1\n",
                    ),
                ],
            ),
            (
                "concepts/synthetic-temple/synthetic-temple-v3.md",
                &[
                    ("\nid: synthetic-shrine-v3\n", "\nid: synthetic-temple-v3\n"),
                    concept,
                    (
                        "\nfrom: synthetic-shrine-v2\n",
                        "\nfrom: synthetic-temple-v2\n",
                    ),
                ],
            ),
        ],
    )?;
    assert_files(&run.project, &expected)?;
    assert!(
        !run.project
            .root()
            .join("concepts/synthetic-shrine")
            .exists()
    );
    Ok(())
}

// AC: CE-20.3
#[test]
fn renaming_a_floor_moves_its_child_folder() -> TestResult {
    let run = renamed("f04-synthetic", "f04-renamed", &["f04-synthetic"])?;
    let new = "world/synthetic-band/f04-renamed.md";
    let hall = "world/synthetic-band/f04-renamed/synthetic-hall.md";
    let where_list = ("\nwhere: [f04-synthetic]\n", "\nwhere: [f04-renamed]\n");
    let expected = expected(
        &run.before,
        &[
            (FLOOR, new),
            (
                "world/synthetic-band/f04-synthetic/",
                "world/synthetic-band/f04-renamed/",
            ),
        ],
        &[
            (new, &[("\nid: f04-synthetic\n", "\nid: f04-renamed\n")]),
            (
                hall,
                &[("\nparent: f04-synthetic\n", "\nparent: f04-renamed\n")],
            ),
            (
                CLUE,
                &[(
                    "\n  - f04-synthetic  # synthetic comment\n",
                    "\n  - f04-renamed  # synthetic comment\n",
                )],
            ),
            (SECOND_CLUE, &[where_list]),
            (
                SCRIPT,
                &[
                    where_list,
                    ("at:: [[f04-synthetic]]", "at:: [[f04-renamed]]"),
                ],
            ),
        ],
    )?;
    assert_files(&run.project, &expected)?;
    for moved in [
        "synthetic-hall.md",
        "synthetic-hall/synthetic-nook.md",
        "notes.txt",
        ".synthetic-hidden",
    ] {
        assert!(
            expected.contains_key(&format!("{RENAMED_FLOOR_FOLDER}/{moved}")),
            "{moved}"
        );
    }
    assert!(!run.project.root().join(FLOOR_FOLDER).exists());
    Ok(())
}

// AC: CE-20.3
#[test]
fn renaming_an_npc_rewrites_voice_and_beat_links() -> TestResult {
    let new = "people/synthetic-folk/synthetic-chanter.md";
    let run = renamed(
        "synthetic-singer",
        "synthetic-chanter",
        &["synthetic-singer"],
    )?;
    let expected = expected(
        &run.before,
        &[(NPC, new)],
        &[
            (
                new,
                &[("\nid: synthetic-singer\n", "\nid: synthetic-chanter\n")],
            ),
            (
                BELIEF,
                &[(
                    "held_by: ['synthetic-singer']",
                    "held_by: ['synthetic-chanter']",
                )],
            ),
            (
                SCRIPT,
                &[
                    (
                        "with:: [[synthetic-singer]]",
                        "with:: [[synthetic-chanter]]",
                    ),
                    (
                        "> [!voice] [[synthetic-singer]] · [[B-960]]",
                        "> [!voice] [[synthetic-chanter]] · [[B-960]]",
                    ),
                ],
            ),
        ],
    )?;
    assert_files(&run.project, &expected)?;
    Ok(())
}

/// Review focus 4: the files of the folder that are not records move byte for byte, dot files
/// included, on every OS; so does a record none of whose links changes.
#[test]
fn a_rename_moves_every_file_of_the_folder() -> TestResult {
    let run = renamed("f04-synthetic", "f04-renamed", &["f04-synthetic"])?;
    let after = files(run.project.root())?;
    for name in [
        "notes.txt",
        ".synthetic-hidden",
        "synthetic-hall/synthetic-nook.md",
    ] {
        let old = format!("{FLOOR_FOLDER}/{name}");
        let new = format!("{RENAMED_FLOOR_FOLDER}/{name}");
        assert_eq!(after.get(&new), run.before.get(&old), "{name}");
        assert!(run.before.contains_key(&old), "{name}");
        assert!(!after.contains_key(&old), "{name}");
    }
    assert_eq!(run.workspace.undo_entries().len(), 1);
    Ok(())
}

// AC: CE-20.3
#[test]
fn a_taken_id_is_refused() -> TestResult {
    let (project, mut workspace) = open("commands")?;
    let error = refused(&project, &mut workspace, &rename("T-960", "T-961"))?;
    assert!(
        matches!(&error, CommandError::IdTaken { id, path }
            if id == "T-961" && path == "lore/truths/T-961.md"),
        "{error:?}"
    );
    Ok(())
}

/// Review focus 4: a file the index does not see (ignored by git) still takes its path.
#[test]
fn an_id_taken_by_an_ignored_file_is_refused() -> TestResult {
    let project = TempProject::copy_of_studio("commands")?;
    let ignored = "lore/truths/T-975.md";
    std::fs::write(project.root().join(".gitignore"), format!("{ignored}\n"))?;
    let text = std::fs::read_to_string(project.root().join(TRUTH))?;
    std::fs::write(project.root().join(ignored), text.replace("T-960", "T-975"))?;
    let mut workspace = Workspace::open(project.root())?;
    assert_eq!(workspace.index().record("T-975")?, Vec::new());

    let error = refused(&project, &mut workspace, &rename("T-960", "T-975"))?;
    assert!(
        matches!(&error, CommandError::IdTaken { id, path } if id == "T-975" && path == ignored),
        "{error:?}"
    );
    Ok(())
}

// AC: CE-20.3
#[test]
fn a_case_only_rename_is_refused() -> TestResult {
    let (project, mut workspace) = open("commands")?;
    let error = refused(&project, &mut workspace, &rename("T-960", "t-960"))?;
    assert!(
        matches!(&error, CommandError::CaseOnly { id, to } if id == "T-960" && to == "t-960"),
        "{error:?}"
    );
    Ok(())
}

// AC: CE-20.3
#[test]
fn an_id_that_breaks_the_kind_rule_is_rejected() -> TestResult {
    let (project, mut workspace) = open("commands")?;
    let error = refused(&project, &mut workspace, &rename("T-960", "x960"))?;
    let CommandError::Rejected { blocking, .. } = &error else {
        return Err(format!("expected Rejected, got {error:?}").into());
    };
    assert!(
        blocking
            .iter()
            .any(|f| f.rule == "schema/id" && f.path == "lore/truths/x960.md"),
        "{blocking:#?}"
    );
    Ok(())
}

// AC: CE-20.2
#[test]
fn undoing_a_rename_restores_every_path() -> TestResult {
    let run = renamed("synthetic-shrine", "synthetic-temple", &CONCEPT_IDS)?;
    let (project, mut workspace) = (run.project, run.workspace);
    let root = project.root().to_path_buf();
    let later = files(&root)?;

    // The entry is found under the new path of a moved file.
    workspace.undo(Scope::Record("concepts/synthetic-temple.md".to_owned()))?;
    assert_eq!(files(&root)?, run.before);
    assert!(!root.join("concepts/synthetic-temple").exists());
    assert!(root.join("concepts/synthetic-shrine").is_dir());
    assert_eq!(cold_dump(&root)?, workspace.index().dump()?);
    assert_eq!(workspace.index().backlinks("synthetic-temple")?, Vec::new());

    // And under the old path.
    workspace.redo(Scope::Record(CONCEPT.to_owned()))?;
    assert_eq!(files(&root)?, later);
    assert!(!root.join("concepts/synthetic-shrine").exists());
    assert_eq!(cold_dump(&root)?, workspace.index().dump()?);

    workspace.undo(Scope::Newest)?;
    assert_eq!(files(&root)?, run.before);
    assert_eq!(cold_dump(&root)?, workspace.index().dump()?);
    Ok(())
}

#[test]
fn a_rename_keeps_a_standing_error_of_the_moved_record() -> TestResult {
    // T-980 has a standing `lore/three-clue` error; it moves with the file and does not block.
    let (project, mut workspace) = open("standing")?;
    let outcome = workspace.run(&rename("T-980", "T-990"))?;
    let new = "lore/truths/T-990.md";
    assert!(
        outcome
            .findings
            .iter()
            .any(|f| f.path == new && f.rule == "lore/three-clue"),
        "{:#?}",
        outcome.findings
    );
    assert_eq!(outcome.introduced, Vec::<Finding>::new());
    assert!(project.root().join(new).is_file());
    let clue = std::fs::read_to_string(project.root().join("lore/clues/C-980.md"))?;
    assert!(clue.contains("\nreveals: T-990\n"), "{clue}");
    Ok(())
}

#[test]
fn an_unknown_id_is_not_found() -> TestResult {
    let (project, mut workspace) = open("commands")?;
    let error = refused(&project, &mut workspace, &rename("T-999", "T-998"))?;
    assert!(
        matches!(&error, CommandError::NotFound { what } if what == "T-999"),
        "{error:?}"
    );
    Ok(())
}

#[test]
fn a_duplicate_id_is_ambiguous() -> TestResult {
    let (project, mut workspace) = open("commands")?;
    // Written after the open: the rename syncs the whole project first.
    let second = "lore/beliefs/T-960.md";
    std::fs::copy(project.root().join(TRUTH), project.root().join(second))?;
    let error = refused(&project, &mut workspace, &rename("T-960", "T-970"))?;
    assert!(
        matches!(&error, CommandError::Ambiguous { id, paths }
            if id == "T-960" && paths == &[second.to_owned(), TRUTH.to_owned()]),
        "{error:?}"
    );
    Ok(())
}

#[test]
fn a_rename_names_itself_and_syncs_everything() {
    use theos_studio::{Command, Reads};
    let command = rename("T-960", "T-970");
    assert_eq!(command.label(), "rename T-960 to T-970");
    assert_eq!(command.reads(), Reads::Everything);
}
