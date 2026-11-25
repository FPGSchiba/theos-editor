//! `theos new`.

use std::path::PathBuf;

use serde_json::Value;
use theos_content::Placement;

use crate::common::{Error, Sandbox, TestResult, fixture, staged_project, stderr, stdout};

/// A copy of the `staged` fixture project with a synthetic truth template, a people record and a
/// faction under it.
fn project(sandbox: &Sandbox) -> Result<PathBuf, Error> {
    let project = sandbox.copy_project(&staged_project(), "new")?;
    sandbox.copy_project(&fixture("theos-studio/tests/fixtures/new"), "new")?;
    Ok(project)
}

fn read(project: &std::path::Path, path: &str) -> std::io::Result<String> {
    std::fs::read_to_string(project.join(path))
}

// AC: CE-17.1
#[test]
fn theos_new_numbers_a_truth_from_the_template() -> TestResult {
    let sandbox = Sandbox::new()?;
    let project = project(&sandbox)?;
    let output = sandbox.run(
        &[
            "new",
            "truth",
            "--title",
            "Synthetic statement B",
            "--set",
            "target=hinted",
        ],
        &project,
    )?;
    assert_eq!(
        output.status.code(),
        Some(0),
        "{output:?} {}",
        stderr(&output)
    );
    let out = stdout(&output);
    assert_eq!(out.lines().last(), Some("lore/truths/T-902.md"), "{out}");
    let text = read(&project, "lore/truths/T-902.md")?;
    assert!(text.contains("id: T-902\n"), "{text}");
    assert!(text.contains("title: Synthetic statement B\n"), "{text}");
    assert!(text.contains("target: hinted\n"), "{text}");
    assert!(
        text.contains("Synthetic statement, stated plainly."),
        "{text}"
    );
    Ok(())
}

// AC: CE-17.1
#[test]
fn theos_new_places_an_npc_under_its_parent() -> TestResult {
    let sandbox = Sandbox::new()?;
    let project = project(&sandbox)?;
    let output = sandbox.run(
        &[
            "new",
            "npc",
            "mother-ilse",
            "--set",
            "parent=synthetic-faction",
            "--title",
            "Mother Ilse",
            "--json",
        ],
        &project,
    )?;
    assert_eq!(
        output.status.code(),
        Some(0),
        "{output:?} {}",
        stderr(&output)
    );
    let view: Value = serde_json::from_str(&stdout(&output))?;
    let folder = Placement::under_parent(
        "people/synthetic-people/synthetic-faction.md",
        "synthetic-faction",
    );
    let path = format!("{folder}/mother-ilse.md");
    assert_eq!(view["path"], path.as_str());
    assert_eq!(view["id"], "mother-ilse");
    assert_eq!(view["introduced"], serde_json::json!([]));
    assert!(read(&project, &path)?.contains("title: Mother Ilse\n"));
    Ok(())
}

// AC: CE-17.1
#[test]
fn theos_new_refuses_an_introduced_error_and_writes_nothing() -> TestResult {
    let sandbox = Sandbox::new()?;
    let project = project(&sandbox)?;
    let clue = [
        "new",
        "clue",
        "--set",
        "reveals=T-901",
        "--set",
        "moves_to=hinted",
        "--set",
        "where=[synthetic-hall]",
        "--set",
        "gate=T1",
        "--set",
        "vector=synthetic",
    ];
    let output = sandbox.run(&clue, &project)?;
    assert_eq!(output.status.code(), Some(2), "{output:?}");
    let out = stdout(&output);
    assert!(
        out.lines()
            .any(|line| line.starts_with("lore/clues/C-901.md:")
                && line.contains(" error lore/never-confirm: ")),
        "{out}"
    );
    assert!(
        stderr(&output).contains("refused: the new record would introduce 1 error"),
        "{}",
        stderr(&output)
    );
    assert!(!project.join("lore/clues/C-901.md").exists());

    // The same refusal as JSON, with the same exit code.
    let mut json = vec!["--json"];
    json.extend(clue);
    let output = sandbox.run(&json, &project)?;
    assert_eq!(output.status.code(), Some(2), "{output:?}");
    let view: Value = serde_json::from_str(&stdout(&output))?;
    assert_eq!(view["path"], "lore/clues/C-901.md");
    assert_eq!(view["introduced"][0]["rule"], "lore/never-confirm");
    assert!(!project.join("lore/clues/C-901.md").exists());
    Ok(())
}

#[test]
fn theos_new_without_a_template_builds_a_skeleton() -> TestResult {
    let sandbox = Sandbox::new()?;
    let project = sandbox.copy_project(&staged_project(), "skeleton")?;
    let output = sandbox.run(&["new", "truth", "--set", "target=hinted"], &project)?;
    assert_eq!(
        output.status.code(),
        Some(0),
        "{output:?} {}",
        stderr(&output)
    );
    assert_eq!(
        read(&project, "lore/truths/T-902.md")?,
        "---\nid: T-902\nkind: truth\ntitle: T-902\nstatus: draft\ntarget: hinted\n---\n"
    );
    Ok(())
}

#[test]
fn unknown_kinds_fields_and_ids_exit_2() -> TestResult {
    let sandbox = Sandbox::new()?;
    let project = project(&sandbox)?;
    for (args, wanted) in [
        (vec!["new", "nonsense"], "unknown kind `nonsense`"),
        (
            vec!["new", "truth", "--set", "colour=red"],
            "no field `colour`",
        ),
        (vec!["new", "truth", "x-1"], "not an id of a truth"),
        (
            vec!["new", "npc", "--set", "parent=synthetic-faction"],
            "give an id",
        ),
        (vec!["new", "npc", "mother-ilse"], "give the parent"),
        (
            vec!["new", "npc", "mother-ilse", "--set", "parent=nobody"],
            "no record `nobody`",
        ),
        (vec!["new", "truth", "--set", "title=a: b"], "title:"),
        (
            vec!["new", "truth", "T-900", "--set", "target=confirmed"],
            "exists already",
        ),
    ] {
        let output = sandbox.run(&args, &project)?;
        assert_eq!(output.status.code(), Some(2), "{args:?}: {output:?}");
        assert!(
            stderr(&output).contains(wanted),
            "{args:?}: {}",
            stderr(&output)
        );
    }
    Ok(())
}

#[test]
fn a_slug_kind_without_an_id_exits_2() -> TestResult {
    let sandbox = Sandbox::new()?;
    let project = project(&sandbox)?;
    let output = sandbox.run(&["new", "people"], &project)?;
    assert_eq!(output.status.code(), Some(2), "{output:?}");
    assert!(
        stderr(&output).contains("give an id"),
        "{}",
        stderr(&output)
    );
    Ok(())
}

#[test]
fn a_set_without_a_field_name_is_a_usage_error() -> TestResult {
    let sandbox = Sandbox::new()?;
    let project = project(&sandbox)?;
    for bad in ["=x", "novalue"] {
        let output = sandbox.run(&["new", "truth", "--set", bad], &project)?;
        assert_eq!(output.status.code(), Some(2), "{bad}: {output:?}");
    }
    Ok(())
}
