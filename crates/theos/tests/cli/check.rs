//! `theos check`, with `--staged`, `--binaries` and `--json`.

use std::path::{Path, PathBuf};
use std::process::Output;

use serde_json::Value;

use crate::common::{
    Error, Sandbox, TestResult, content_project, fixture, staged_project, stderr, stdout, variant,
};

/// A copy of the staged fixture project with the never-confirm violation `C-901` added.
fn violating(sandbox: &Sandbox) -> Result<PathBuf, Error> {
    let project = sandbox.copy_project(&staged_project(), "violating")?;
    put_violation(&project)?;
    Ok(project)
}

fn put_violation(project: &Path) -> Result<PathBuf, Error> {
    let path = project.join("lore").join("clues").join("C-901.md");
    std::fs::write(&path, variant("C-901.md")?)?;
    Ok(path)
}

fn json(output: &Output) -> Result<Value, serde_json::Error> {
    serde_json::from_str(&stdout(output))
}

// AC: CE-17.1
#[test]
fn theos_check_prints_finding_lines_and_exits_1_on_errors() -> TestResult {
    let sandbox = Sandbox::new()?;
    let project = violating(&sandbox)?;
    let output = sandbox.run(&["check"], &project)?;
    assert_eq!(output.status.code(), Some(1), "{output:?}");
    let out = stdout(&output);
    assert!(
        out.lines()
            .any(|line| line.starts_with("lore/clues/C-901.md:")
                && line.contains(" error lore/never-confirm: ")),
        "{out}"
    );
    let err = stderr(&output);
    assert!(err.contains("1 error, 1 warning"), "{err}");
    assert!(
        err.contains("binaries not checked: no binaries folder set"),
        "{err}"
    );
    assert!(!out.contains("binaries not checked"), "{out}");
    Ok(())
}

// AC: CE-17.1
#[test]
fn theos_check_json_has_findings_counts_and_binaries() -> TestResult {
    let sandbox = Sandbox::new()?;
    let project = violating(&sandbox)?;
    let output = sandbox.run(&["check", "--json"], &project)?;
    assert_eq!(output.status.code(), Some(1), "{output:?}");
    let value = json(&output)?;
    let object = value.as_object().ok_or("an object")?;
    for key in ["findings", "errors", "warnings", "binaries"] {
        assert!(object.contains_key(key), "{key}");
    }
    assert_eq!(value["errors"], 1);
    assert_eq!(value["warnings"], 1);
    assert_eq!(
        value["binaries"],
        serde_json::json!({
            "checked": false, "folder": null, "reason": "no binaries folder set"
        })
    );
    let findings = value["findings"].as_array().ok_or("findings")?;
    let never = findings
        .iter()
        .find(|finding| finding["rule"] == "lore/never-confirm")
        .ok_or("never-confirm finding")?;
    for key in [
        "rule", "severity", "path", "id", "line", "column", "span", "message",
    ] {
        assert!(never.get(key).is_some(), "{key}");
    }
    assert_eq!(never["path"], "lore/clues/C-901.md");
    assert_eq!(never["severity"], "error");
    Ok(())
}

// AC: CE-17.1
#[test]
fn theos_check_json_keeps_the_notes_on_stderr() -> TestResult {
    let sandbox = Sandbox::new()?;
    let project = sandbox.copy_project(&staged_project(), "clean")?;
    let output = sandbox.run(&["check", "--json"], &project)?;
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    json(&output)?;
    let err = stderr(&output);
    assert!(
        err.contains("binaries not checked: no binaries folder set"),
        "{err}"
    );
    assert!(!err.contains("0 errors"), "no summary line: {err}");
    Ok(())
}

// AC: CE-17.1
#[test]
fn theos_check_reads_the_binaries_folder_from_user_settings() -> TestResult {
    let sandbox = Sandbox::new()?;
    let project = sandbox.copy_project(&content_project("binaries"), "binaries")?;
    let folder = std::path::absolute(fixture("theos-content/tests/fixtures/binaries"))?;
    let setting = format!(
        "binaries_root = {}\n",
        serde_json::to_string(folder.to_str().ok_or("path")?)?
    );
    std::fs::write(sandbox.config().join("settings.toml"), setting)?;
    let output = sandbox.run(&["check", "--json"], &project)?;
    let value = json(&output)?;
    assert_eq!(value["binaries"]["checked"], true, "{value}");
    assert_eq!(
        value["binaries"]["folder"],
        Value::String(folder.to_string_lossy().into_owned())
    );
    let binary_warnings = value["findings"]
        .as_array()
        .ok_or("findings")?
        .iter()
        .filter(|finding| {
            finding["rule"]
                .as_str()
                .is_some_and(|rule| rule.starts_with("binary/"))
        })
        .count();
    assert!(binary_warnings > 0, "{value}");

    // The flag wins over the setting: an empty folder holds none of the files.
    let other = sandbox.work().join("empty-binaries");
    std::fs::create_dir(&other)?;
    let with_flag = sandbox.run(
        &[
            "check",
            "--json",
            "--binaries",
            other.to_str().ok_or("path")?,
        ],
        &project,
    )?;
    let flagged = json(&with_flag)?;
    assert_eq!(
        flagged["binaries"]["folder"],
        Value::String(other.to_string_lossy().into_owned())
    );
    Ok(())
}

#[test]
fn theos_check_of_a_clean_project_exits_0() -> TestResult {
    let sandbox = Sandbox::new()?;
    let project = sandbox.copy_project(&staged_project(), "clean")?;
    let output = sandbox.run(&["check"], &project)?;
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    let out = stdout(&output);
    assert!(out.contains("lore/three-clue"), "the known warning: {out}");
    assert!(
        stderr(&output).contains("0 errors, 1 warning"),
        "{output:?}"
    );
    Ok(())
}

#[test]
fn an_unreadable_user_setting_is_a_note_not_a_failure() -> TestResult {
    let sandbox = Sandbox::new()?;
    let project = sandbox.copy_project(&staged_project(), "clean")?;
    std::fs::write(sandbox.config().join("settings.toml"), "not = [valid")?;
    let output = sandbox.run(&["check"], &project)?;
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    let err = stderr(&output);
    assert!(err.contains("binaries not checked: "), "{err}");
    assert!(err.contains("settings.toml"), "{err}");
    Ok(())
}

#[test]
fn theos_check_finds_the_project_above_the_current_folder() -> TestResult {
    let sandbox = Sandbox::new()?;
    let project = violating(&sandbox)?;
    let output = sandbox.run(&["check"], &project.join("lore").join("clues"))?;
    assert_eq!(output.status.code(), Some(1), "{output:?}");
    assert!(stdout(&output).contains("lore/clues/C-901.md:"));
    Ok(())
}

// AC: CE-17.1
#[test]
fn theos_check_outside_a_project_exits_2() -> TestResult {
    let sandbox = Sandbox::new()?;
    let output = sandbox.run(&["check"], &sandbox.work())?;
    assert_eq!(output.status.code(), Some(2), "{output:?}");
    assert_eq!(output.stdout, Vec::<u8>::new());
    assert!(
        stderr(&output).starts_with("error: no theos.toml"),
        "{output:?}"
    );
    Ok(())
}

#[test]
fn staged_and_binaries_together_exit_2() -> TestResult {
    let sandbox = Sandbox::new()?;
    let project = sandbox.copy_project(&staged_project(), "clean")?;
    let output = sandbox.run(&["check", "--staged", "--binaries", "somewhere"], &project)?;
    assert_eq!(output.status.code(), Some(2), "{output:?}");
    assert!(stderr(&output).contains("--staged"), "{output:?}");
    Ok(())
}

// AC: CE-17.2
#[test]
fn theos_check_staged_judges_the_index_not_the_working_tree() -> TestResult {
    let sandbox = Sandbox::new()?;
    let project = sandbox.copy_project(&staged_project(), "repo")?;
    sandbox.commit_all(&project)?;

    // Untracked on disk only: the snapshot is clean.
    let violation = put_violation(&project)?;
    let clean = sandbox.run(&["check", "--staged"], &project)?;
    assert_eq!(clean.status.code(), Some(0), "{clean:?}");
    assert!(
        stderr(&clean).contains("staged snapshot: no errors, 1 warning"),
        "{clean:?}"
    );
    let value = json(&sandbox.run(&["check", "--staged", "--json"], &project)?)?;
    assert!(value.get("binaries").is_none(), "{value}");
    assert_eq!(value["changed"], serde_json::json!([]));

    // Staged, then deleted on disk: the index still holds the violation.
    sandbox.git(&["add", "lore/clues/C-901.md"], &project)?;
    std::fs::remove_file(&violation)?;
    let refused = sandbox.run(&["check", "--staged"], &project)?;
    assert_eq!(refused.status.code(), Some(1), "{refused:?}");
    assert!(
        stdout(&refused).contains("lore/clues/C-901.md:"),
        "{refused:?}"
    );
    assert!(
        stderr(&refused).contains("commit refused: 1 error in the staged snapshot"),
        "{refused:?}"
    );
    let value = json(&sandbox.run(&["check", "--staged", "--json"], &project)?)?;
    assert_eq!(value["changed"], serde_json::json!(["lore/clues/C-901.md"]));
    assert_eq!(value["errors"], 1);
    Ok(())
}
