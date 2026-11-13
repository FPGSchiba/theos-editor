//! `theos hook install`: the hook it writes, its refusals and `--force`.

use std::path::PathBuf;

use serde_json::Value;

use crate::common::{Sandbox, TestResult, staged_project, stderr, stdout};

/// A repository whose top is the staged fixture project, committed. Returns the top.
fn repository(sandbox: &Sandbox) -> Result<PathBuf, crate::common::Error> {
    let top = sandbox.copy_project(&staged_project(), "repo")?;
    sandbox.commit_all(&top)?;
    Ok(top)
}

fn hook_of(top: &std::path::Path) -> PathBuf {
    top.join(".git").join("hooks").join("pre-commit")
}

// AC: CE-17.1
#[test]
fn theos_hook_install_writes_the_hook() -> TestResult {
    let sandbox = Sandbox::new()?;
    let top = repository(&sandbox)?;
    let output = sandbox.run(&["hook", "install"], &top)?;
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    let text = std::fs::read_to_string(hook_of(&top))?;
    assert!(text.starts_with("#!/bin/sh\n"), "{text}");
    assert!(text.contains("# theos pre-commit hook"), "{text}");
    assert!(
        text.lines()
            .any(|line| line == "exec \"$theos\" check --staged '.'"),
        "{text}"
    );
    let printed = stdout(&output);
    assert_eq!(
        std::fs::canonicalize(printed.trim_end())?,
        std::fs::canonicalize(hook_of(&top))?
    );
    Ok(())
}

// AC: CE-17.1
#[test]
fn theos_hook_install_json() -> TestResult {
    let sandbox = Sandbox::new()?;
    let top = repository(&sandbox)?;
    let output = sandbox.run(&["hook", "install", "--json"], &top)?;
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    let view: Value = serde_json::from_str(&stdout(&output))?;
    assert!(
        view["hook"]
            .as_str()
            .is_some_and(|hook| hook.ends_with("pre-commit"))
    );
    assert_eq!(view["replaced"], false);
    assert_eq!(view["backup"], Value::Null);
    let again = sandbox.run(&["hook", "install", "--json"], &top)?;
    let view: Value = serde_json::from_str(&stdout(&again))?;
    assert_eq!(view["replaced"], true);
    Ok(())
}

// AC: CE-17.1
#[test]
fn theos_hook_install_refuses_a_foreign_hook_with_2() -> TestResult {
    let sandbox = Sandbox::new()?;
    let top = repository(&sandbox)?;
    std::fs::create_dir_all(top.join(".git").join("hooks"))?;
    std::fs::write(hook_of(&top), "#!/bin/sh\necho mine\n")?;
    let output = sandbox.run(&["hook", "install"], &top)?;
    assert_eq!(output.status.code(), Some(2), "{output:?}");
    assert!(
        stderr(&output).contains("pre-commit"),
        "{}",
        stderr(&output)
    );
    assert_eq!(
        std::fs::read_to_string(hook_of(&top))?,
        "#!/bin/sh\necho mine\n"
    );
    let forced = sandbox.run(&["hook", "install", "--force", "--json"], &top)?;
    assert_eq!(
        forced.status.code(),
        Some(0),
        "{forced:?} {}",
        stderr(&forced)
    );
    let view: Value = serde_json::from_str(&stdout(&forced))?;
    assert!(
        view["backup"]
            .as_str()
            .is_some_and(|backup| backup.ends_with("pre-commit.before-theos"))
    );
    assert_eq!(
        std::fs::read_to_string(
            top.join(".git")
                .join("hooks")
                .join("pre-commit.before-theos")
        )?,
        "#!/bin/sh\necho mine\n"
    );
    Ok(())
}

// AC: CE-17.1
#[test]
fn theos_hook_install_of_a_folder_inside_a_project_exits_2() -> TestResult {
    let sandbox = Sandbox::new()?;
    let top = repository(&sandbox)?;
    let output = sandbox.run(&["hook", "install", "lore"], &top)?;
    assert_eq!(output.status.code(), Some(2), "{output:?}");
    assert!(
        stderr(&output).contains("is not a content project"),
        "{}",
        stderr(&output)
    );
    assert!(!hook_of(&top).exists());
    Ok(())
}

// AC: CE-17.1
#[test]
fn theos_hook_install_finds_the_project_above_the_current_folder() -> TestResult {
    let sandbox = Sandbox::new()?;
    let top = repository(&sandbox)?;
    let output = sandbox.run(&["hook", "install"], &top.join("lore").join("clues"))?;
    assert_eq!(
        output.status.code(),
        Some(0),
        "{output:?} {}",
        stderr(&output)
    );
    let text = std::fs::read_to_string(hook_of(&top))?;
    assert!(
        text.lines()
            .any(|line| line == "exec \"$theos\" check --staged '.'"),
        "{text}"
    );
    let given = sandbox.run(&["hook", "install", "--json", "."], &top)?;
    assert_eq!(given.status.code(), Some(0), "{given:?}");
    let view: Value = serde_json::from_str(&stdout(&given))?;
    assert_eq!(view["replaced"], true);
    Ok(())
}

#[test]
fn a_project_in_a_subfolder_names_its_folder() -> TestResult {
    let sandbox = Sandbox::new()?;
    let project = sandbox.copy_project(&staged_project(), "repo/[draft] Inhalt ä")?;
    let top = sandbox.work().join("repo");
    sandbox.git(&["init", "-q", "-b", "main"], &top)?;
    let output = sandbox.run(&["hook", "install"], &project)?;
    assert_eq!(
        output.status.code(),
        Some(0),
        "{output:?} {}",
        stderr(&output)
    );
    let text = std::fs::read_to_string(hook_of(&top))?;
    assert!(
        text.lines()
            .any(|line| line == "exec \"$theos\" check --staged '[draft] Inhalt ä'"),
        "{text}"
    );
    Ok(())
}
