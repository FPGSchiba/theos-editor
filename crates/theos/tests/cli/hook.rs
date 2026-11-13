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

fn merge_hook_of(top: &std::path::Path) -> PathBuf {
    top.join(".git").join("hooks").join("pre-merge-commit")
}

/// The keys of a JSON object, sorted.
fn keys(value: &Value) -> Vec<&str> {
    let mut keys: Vec<&str> = value
        .as_object()
        .map(|object| object.keys().map(String::as_str).collect())
        .unwrap_or_default();
    keys.sort_unstable();
    keys
}

// AC: CE-17.1
#[test]
fn theos_hook_install_prints_both_hooks() -> TestResult {
    let sandbox = Sandbox::new()?;
    let top = repository(&sandbox)?;
    let output = sandbox.run(&["hook", "install"], &top)?;
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    let printed = stdout(&output);
    let lines: Vec<&str> = printed.lines().collect();
    assert_eq!(lines.len(), 2, "{printed}");
    assert_eq!(
        std::fs::canonicalize(lines[0])?,
        std::fs::canonicalize(hook_of(&top))?
    );
    assert_eq!(
        std::fs::canonicalize(lines[1])?,
        std::fs::canonicalize(merge_hook_of(&top))?
    );
    assert_eq!(
        std::fs::read_to_string(merge_hook_of(&top))?,
        std::fs::read_to_string(hook_of(&top))?
    );
    Ok(())
}

// AC: CE-17.1
#[test]
fn theos_hook_install_json_names_the_merge_hook() -> TestResult {
    let sandbox = Sandbox::new()?;
    let top = repository(&sandbox)?;
    std::fs::create_dir_all(top.join(".git").join("hooks"))?;
    std::fs::write(merge_hook_of(&top), "#!/bin/sh\necho mine\n")?;
    let output = sandbox.run(&["hook", "install", "--json", "--force"], &top)?;
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    let view: Value = serde_json::from_str(&stdout(&output))?;
    assert_eq!(
        keys(&view),
        ["backup", "hook", "pre_merge_commit", "replaced"]
    );
    assert_eq!(view["backup"], Value::Null);
    let merge = &view["pre_merge_commit"];
    assert_eq!(keys(merge), ["backup", "hook", "replaced"]);
    let hook = merge["hook"].as_str().ok_or("no merge hook path")?;
    assert!(!hook.contains(foreign_separator()), "{hook}");
    assert_eq!(
        std::fs::canonicalize(hook)?,
        std::fs::canonicalize(merge_hook_of(&top))?
    );
    assert_eq!(merge["replaced"], false);
    let backup = merge["backup"].as_str().ok_or("no backup named")?;
    assert!(
        backup.ends_with("pre-merge-commit.before-theos"),
        "{backup}"
    );
    assert_eq!(std::fs::read_to_string(backup)?, "#!/bin/sh\necho mine\n");

    let again = sandbox.run(&["hook", "install", "--json"], &top)?;
    let view: Value = serde_json::from_str(&stdout(&again))?;
    assert_eq!(view["pre_merge_commit"]["replaced"], true);
    assert_eq!(view["pre_merge_commit"]["backup"], Value::Null);
    Ok(())
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
    let first = printed.lines().next().ok_or("nothing printed")?;
    assert_eq!(
        std::fs::canonicalize(first)?,
        std::fs::canonicalize(hook_of(&top))?
    );
    Ok(())
}

/// The separator that is not this platform's: `/` on Windows, `\` elsewhere.
fn foreign_separator() -> char {
    if std::path::MAIN_SEPARATOR == '/' {
        '\\'
    } else {
        '/'
    }
}

// The path is printed as written, not canonicalized, so git's `C:/…` must not show through.
// AC: CE-17.1
#[test]
fn theos_hook_install_prints_the_path_with_one_separator() -> TestResult {
    let sandbox = Sandbox::new()?;
    let top = repository(&sandbox)?;
    let output = sandbox.run(&["hook", "install"], &top)?;
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    let printed = stdout(&output);
    assert!(!printed.contains(foreign_separator()), "{printed}");
    assert!(printed.ends_with("pre-merge-commit\n"), "{printed}");
    let again = sandbox.run(&["hook", "install", "--json"], &top)?;
    let view: Value = serde_json::from_str(&stdout(&again))?;
    let hook = view["hook"].as_str().ok_or("no hook path")?;
    assert!(!hook.contains(foreign_separator()), "{hook}");
    let merge = view["pre_merge_commit"]["hook"]
        .as_str()
        .ok_or("no merge hook path")?;
    assert_eq!(format!("{hook}\n{merge}\n"), printed);
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
fn theos_hook_install_force_names_the_backup() -> TestResult {
    let sandbox = Sandbox::new()?;
    let top = repository(&sandbox)?;
    std::fs::create_dir_all(top.join(".git").join("hooks"))?;
    std::fs::write(hook_of(&top), "#!/bin/sh\necho mine\n")?;
    std::fs::write(merge_hook_of(&top), "#!/bin/sh\necho merge\n")?;
    let output = sandbox.run(&["hook", "install", "--force"], &top)?;
    assert_eq!(
        output.status.code(),
        Some(0),
        "{output:?} {}",
        stderr(&output)
    );
    let printed = stdout(&output);
    assert!(!printed.contains(foreign_separator()), "{printed}");
    let lines: Vec<&str> = printed.lines().collect();
    assert_eq!(lines.len(), 4, "{printed}");
    assert!(lines[0].ends_with("pre-commit"), "{printed}");
    assert!(lines[1].ends_with("pre-merge-commit"), "{printed}");
    for (line, name, text) in [
        (
            lines[2],
            "pre-commit.before-theos",
            "#!/bin/sh\necho mine\n",
        ),
        (
            lines[3],
            "pre-merge-commit.before-theos",
            "#!/bin/sh\necho merge\n",
        ),
    ] {
        let backup = line
            .strip_prefix("kept the earlier hook as ")
            .ok_or("the backup is not named")?;
        assert!(backup.ends_with(name), "{printed}");
        assert_eq!(std::fs::read_to_string(backup)?, text);
    }
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
